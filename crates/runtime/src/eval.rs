use crate::value::{Instance, Value};
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use syntra_diagnostics::Span;
use syntra_syntax::ast::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Runtime error at span {span:?}: {message}")]
    General { message: String, span: Span },
}

enum ControlFlow {
    Return(Value),
    Error(RuntimeError),
}

#[derive(Clone)]
pub struct Interpreter {
    classes: HashMap<String, ClassDecl>,
    functions: HashMap<String, FunctionDecl>,
    scopes: Vec<HashMap<String, Value>>,
    current_instance: Option<Arc<Mutex<Instance>>>,
    args: Vec<String>,
    pub profiler: crate::profile::SharedProfileCollector,
    pub debug_state: Option<Arc<Mutex<crate::debug::DebugHookState>>>,
    pub sources: HashMap<String, String>,
    pub current_file: String,
    pub current_fn: String,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            functions: HashMap::new(),
            scopes: vec![HashMap::new()],
            current_instance: None,
            args: Vec::new(),
            profiler: crate::profile::SharedProfileCollector::new(),
            debug_state: None,
            sources: HashMap::new(),
            current_file: String::new(),
            current_fn: String::new(),
        }
    }

    pub fn attach_debugger(&mut self, state: Arc<Mutex<crate::debug::DebugHookState>>) {
        self.debug_state = Some(state);
    }

    pub fn set_source(&mut self, file: &str, content: &str) {
        self.sources.insert(file.to_string(), content.to_string());
        if self.current_file.is_empty() {
            self.current_file = file.to_string();
        }
    }

    pub fn offset_to_line(&self, offset: usize) -> usize {
        let text = self
            .sources
            .get(&self.current_file)
            .or_else(|| self.sources.values().next());
        if let Some(src) = text {
            let mut line = 1;
            for (i, c) in src.char_indices() {
                if i >= offset {
                    break;
                }
                if c == '\n' {
                    line += 1;
                }
            }
            line
        } else {
            1
        }
    }

    fn record_fn_enter(&mut self, name: &str) {
        self.current_fn = name.to_string();
        if let Some(ref dbg) = self.debug_state {
            dbg.lock()
                .unwrap()
                .call_stack
                .push((name.to_string(), self.current_file.clone(), 1));
        }
    }

    fn record_fn_exit(&mut self, _name: &str) {
        if let Some(ref dbg) = self.debug_state {
            let mut state = dbg.lock().unwrap();
            state.call_stack.pop();
            if let Some(top) = state.call_stack.last() {
                self.current_fn = top.0.clone();
            } else {
                self.current_fn.clear();
            }
        }
    }

    pub fn eval_expr_str(&mut self, expr_str: &str) -> Result<Value, String> {
        let trimmed = expr_str.trim();
        // 1. Direct variable lookup
        if let Some(val) = self.get_variable(trimmed) {
            return Ok(val);
        }
        // 2. Parse and evaluate expression
        let code = format!("func __dbg_eval() {{ return {trimmed}; }}");
        if let Ok(prog) = syntra_syntax::parse(&code) {
            for item in &prog.items {
                if let syntra_syntax::ast::Item::Function(f) = item {
                    if let Some(syntra_syntax::ast::Stmt::Return { value: Some(e), .. }) =
                        f.body.statements.first()
                    {
                        return self.eval_expr(e).map_err(|err| format!("{err:?}"));
                    }
                }
            }
        }
        Err(format!("Could not evaluate expression '{expr_str}'"))
    }

    fn check_debug_hook(&mut self, stmt: &Stmt) -> Result<(), ControlFlow> {
        let debug_state_arc = match &self.debug_state {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        let span = stmt.span();
        let line = self.offset_to_line(span.start);
        let cur_file = self.current_file.clone();
        let cur_fn = self.current_fn.clone();
        let cur_depth = self.scopes.len();

        let mut should_pause = false;
        let mut pause_reason = crate::debug::PauseReason::Step;

        {
            let mut state = debug_state_arc.lock().unwrap();
            if let Some(top) = state.call_stack.last_mut() {
                top.2 = line;
            }

            // 1. Check if breakpoint hit
            let base_file = std::path::Path::new(&cur_file)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(&cur_file);

            let hit_bp = state
                .breakpoints
                .iter()
                .any(|(f, lines)| (f == &cur_file || f == base_file) && lines.contains(&line));

            if hit_bp {
                should_pause = true;
                pause_reason = crate::debug::PauseReason::Breakpoint {
                    file: cur_file.clone(),
                    line,
                };
            } else {
                // 2. Check step mode
                match state.step_mode {
                    crate::debug::StepMode::Continue => {}
                    crate::debug::StepMode::StepInto => {
                        should_pause = true;
                        pause_reason = crate::debug::PauseReason::Step;
                    }
                    crate::debug::StepMode::StepOver(target_depth) => {
                        if cur_depth <= target_depth {
                            should_pause = true;
                            pause_reason = crate::debug::PauseReason::Step;
                        }
                    }
                    crate::debug::StepMode::StepOut(target_depth) => {
                        if cur_depth < target_depth {
                            should_pause = true;
                            pause_reason = crate::debug::PauseReason::Step;
                        }
                    }
                }
            }
        }

        if !should_pause {
            return Ok(());
        }

        // Send Paused event
        {
            let state = debug_state_arc.lock().unwrap();
            let _ = state.event_tx.send(crate::debug::DebugEvent::Paused {
                reason: pause_reason,
                file: cur_file.clone(),
                line,
                fn_name: cur_fn.clone(),
            });
        }

        // Wait for user/DAP commands while paused
        loop {
            let cmd = {
                let state = debug_state_arc.lock().unwrap();
                state.cmd_rx.recv().map_err(|_| {
                    ControlFlow::Error(RuntimeError::General {
                        message: "Debug connection closed".to_string(),
                        span,
                    })
                })?
            };

            match cmd {
                crate::debug::DebugCommand::Continue => {
                    let mut state = debug_state_arc.lock().unwrap();
                    state.step_mode = crate::debug::StepMode::Continue;
                    break;
                }
                crate::debug::DebugCommand::StepInto => {
                    let mut state = debug_state_arc.lock().unwrap();
                    state.step_mode = crate::debug::StepMode::StepInto;
                    break;
                }
                crate::debug::DebugCommand::StepOver => {
                    let mut state = debug_state_arc.lock().unwrap();
                    state.step_mode = crate::debug::StepMode::StepOver(cur_depth);
                    break;
                }
                crate::debug::DebugCommand::StepOut => {
                    let mut state = debug_state_arc.lock().unwrap();
                    state.step_mode = crate::debug::StepMode::StepOut(cur_depth);
                    break;
                }
                crate::debug::DebugCommand::SetBreakpoint { file, line } => {
                    let mut state = debug_state_arc.lock().unwrap();
                    state.breakpoints.entry(file).or_default().insert(line);
                }
                crate::debug::DebugCommand::RemoveBreakpoint { file, line } => {
                    let mut state = debug_state_arc.lock().unwrap();
                    if let Some(lines) = state.breakpoints.get_mut(&file) {
                        lines.remove(&line);
                    }
                }
                crate::debug::DebugCommand::GetStackTrace(reply) => {
                    let state = debug_state_arc.lock().unwrap();
                    let mut frames = Vec::new();
                    for (frame_id, (f_name, f_file, f_line)) in
                        state.call_stack.iter().rev().enumerate()
                    {
                        let mut vars = HashMap::new();
                        if frame_id == 0 {
                            for scope in &self.scopes {
                                for (k, v) in scope {
                                    vars.insert(k.clone(), v.clone());
                                }
                            }
                        }
                        frames.push(crate::debug::StackFrame {
                            id: frame_id,
                            fn_name: f_name.clone(),
                            file: f_file.clone(),
                            line: *f_line,
                            variables: vars,
                        });
                    }
                    let _ = reply.send(frames);
                }
                crate::debug::DebugCommand::GetVariables { frame_id: _, reply } => {
                    let mut vars = HashMap::new();
                    for scope in &self.scopes {
                        for (k, v) in scope {
                            vars.insert(k.clone(), v.clone());
                        }
                    }
                    let _ = reply.send(vars);
                }
                crate::debug::DebugCommand::Evaluate {
                    expr,
                    frame_id: _,
                    reply,
                } => {
                    let res = self.eval_expr_str(&expr);
                    let _ = reply.send(res);
                }
                crate::debug::DebugCommand::Quit => {
                    return Err(ControlFlow::Error(RuntimeError::General {
                        message: "Debugger aborted execution".to_string(),
                        span,
                    }));
                }
            }
        }

        Ok(())
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }

    pub fn generate_class_schema(&self, class_name: &str) -> String {
        let cls = match self.classes.get(class_name) {
            Some(c) => c,
            None => return "{\"type\":\"object\",\"properties\":{}}".to_string(),
        };

        let mut prop_entries = Vec::new();
        let mut required = Vec::new();

        for m in &cls.members {
            if let ClassMember::Field(f) = m {
                let ty_schema = match f.ty.name.as_str() {
                    "int" => "{\"type\":\"integer\",\"format\":\"int64\"}".to_string(),
                    "float" => "{\"type\":\"number\",\"format\":\"double\"}".to_string(),
                    "bool" => "{\"type\":\"boolean\"}".to_string(),
                    "string" => "{\"type\":\"string\"}".to_string(),
                    other => {
                        if f.ty.is_array {
                            let item_ty = match other {
                                "int" => "{\"type\":\"integer\"}".to_string(),
                                "float" => "{\"type\":\"number\"}".to_string(),
                                "bool" => "{\"type\":\"boolean\"}".to_string(),
                                "string" => "{\"type\":\"string\"}".to_string(),
                                custom => {
                                    format!("{{\"$ref\":\"#/components/schemas/{custom}\"}}")
                                }
                            };
                            format!("{{\"type\":\"array\",\"items\":{item_ty}}}")
                        } else {
                            format!("{{\"$ref\":\"#/components/schemas/{other}\"}}")
                        }
                    }
                };

                prop_entries.push(format!("\"{}\": {}", f.name, ty_schema));
                required.push(format!("\"{}\"", f.name));
            }
        }

        prop_entries.sort();
        required.sort();

        format!(
            "{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            prop_entries.join(", "),
            required.join(", ")
        )
    }

    pub fn run_program(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        // Register top-level items
        for item in &program.items {
            self.register_item(item);
        }

        // Look for main function
        let main_func =
            self.functions
                .get("main")
                .cloned()
                .ok_or_else(|| RuntimeError::General {
                    message: "No 'main' function found in program".to_string(),
                    span: Span::default(),
                })?;

        let result = match self.call_function(&main_func, Vec::new()) {
            Ok(val) => Ok(val),
            Err(ControlFlow::Return(val)) => Ok(val),
            Err(ControlFlow::Error(err)) => Err(err),
        };

        if let Some(ref dbg) = self.debug_state {
            let code = match &result {
                Ok(Value::Int(c)) => *c,
                Ok(_) => 0,
                Err(_) => 1,
            };
            let _ = dbg
                .lock()
                .unwrap()
                .event_tx
                .send(crate::debug::DebugEvent::Terminated { exit_code: code });
        }

        result
    }

    fn register_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => {
                self.functions.insert(f.name.clone(), f.clone());
            }
            Item::Class(c) => {
                self.classes.insert(c.name.clone(), c.clone());
            }
            Item::Namespace(ns) => {
                for sub in &ns.items {
                    match sub {
                        Item::Function(f) => {
                            let mut namespaced = f.clone();
                            namespaced.name = format!("{}_{}", ns.name, f.name);
                            self.functions.insert(namespaced.name.clone(), namespaced);
                        }
                        Item::Class(c) => {
                            let mut namespaced = c.clone();
                            namespaced.name = format!("{}_{}", ns.name, c.name);
                            self.classes.insert(namespaced.name.clone(), namespaced);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn define_variable(&mut self, name: &str, val: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), val);
        }
    }

    fn set_variable(&mut self, name: &str, val: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), val);
                return;
            }
        }
        // If not found in outer scopes, insert into innermost scope
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), val);
        }
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    fn call_function(
        &mut self,
        func: &FunctionDecl,
        args: Vec<Value>,
    ) -> Result<Value, ControlFlow> {
        self.profiler.record_call_enter(&func.name);
        self.record_fn_enter(&func.name);
        if func.is_async {
            let mut sub_interp = self.clone();
            let func_clone = func.clone();
            let handle = std::thread::spawn(move || {
                sub_interp.enter_scope();
                for (param, arg) in func_clone.params.iter().zip(args) {
                    sub_interp.define_variable(&param.name, arg);
                }
                let res = sub_interp.execute_block(&func_clone.body);
                sub_interp.exit_scope();
                sub_interp.profiler.record_call_exit(&func_clone.name);
                match res {
                    Ok(()) => Value::Void,
                    Err(ControlFlow::Return(val)) => val,
                    Err(_) => Value::Void,
                }
            });
            return Ok(Value::Task(crate::value::TaskHandle::new(handle)));
        }

        self.enter_scope();

        for (param, arg) in func.params.iter().zip(args) {
            self.define_variable(&param.name, arg);
        }

        let res = self.execute_block(&func.body);
        self.exit_scope();
        self.record_fn_exit(&func.name);
        self.profiler.record_call_exit(&func.name);

        match res {
            Ok(()) => Ok(Value::Void),
            Err(ControlFlow::Return(val)) => Ok(val),
            Err(err) => Err(err),
        }
    }

    pub fn invoke_lambda(
        &mut self,
        lambda: &Value,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Value::Lambda {
            params,
            body_expr,
            body_block,
            captured_scopes,
        } = lambda
        {
            let mut run_scopes = captured_scopes.clone();
            let mut call_scope = HashMap::new();
            for (param, arg) in params.iter().zip(args.into_iter()) {
                call_scope.insert(param.clone(), arg);
            }
            run_scopes.push(call_scope);
            let prev_scopes = std::mem::replace(&mut self.scopes, run_scopes);

            let res = if let Some(expr) = body_expr {
                self.eval_expr(expr)
            } else if let Some(block) = body_block {
                match self.execute_block(block) {
                    Ok(()) => Ok(Value::Void),
                    Err(ControlFlow::Return(v)) => Ok(v),
                    Err(ControlFlow::Error(e)) => Err(e),
                }
            } else {
                Ok(Value::Void)
            };

            let executed_scopes = std::mem::replace(&mut self.scopes, prev_scopes);
            for (idx, scope) in executed_scopes.into_iter().enumerate() {
                if idx < self.scopes.len() {
                    for (k, v) in scope {
                        if self.scopes[idx].contains_key(&k) {
                            self.scopes[idx].insert(k, v);
                        }
                    }
                }
            }
            res
        } else {
            Err(RuntimeError::General {
                message: format!("Attempted to invoke non-lambda value '{lambda}'"),
                span,
            })
        }
    }

    fn execute_block(&mut self, block: &Block) -> Result<(), ControlFlow> {
        for stmt in &block.statements {
            self.execute_statement(stmt)?;
        }
        Ok(())
    }

    fn execute_statement(&mut self, stmt: &Stmt) -> Result<(), ControlFlow> {
        self.check_debug_hook(stmt)?;
        match stmt {
            Stmt::VarDecl { name, init, .. } => {
                let val = self.eval_expr(init).map_err(ControlFlow::Error)?;
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(name.clone(), val);
                }
            }
            Stmt::Assignment {
                target,
                value,
                span,
            } => {
                let val = self.eval_expr(value).map_err(ControlFlow::Error)?;
                match target {
                    Expr::Variable(name, _) => {
                        self.set_variable(name, val);
                    }
                    Expr::MemberAccess { object, member, .. } => {
                        let obj_val = self.eval_expr(object).map_err(ControlFlow::Error)?;
                        if let Value::Object(inst) = obj_val {
                            inst.lock().unwrap().fields.insert(member.clone(), val);
                        } else {
                            return Err(ControlFlow::Error(RuntimeError::General {
                                message: format!("Cannot set property '{member}' on non-object"),
                                span: *span,
                            }));
                        }
                    }
                    Expr::Index {
                        array,
                        index,
                        span: idx_span,
                    } => {
                        let arr_val = self.eval_expr(array).map_err(ControlFlow::Error)?;
                        let idx_val = self.eval_expr(index).map_err(ControlFlow::Error)?;
                        if let (Value::Array(arr), Value::Int(i)) = (&arr_val, &idx_val) {
                            let mut borrowed = arr.lock().unwrap();
                            if *i < 0 || (*i as usize) >= borrowed.len() {
                                return Err(ControlFlow::Error(RuntimeError::General {
                                    message: format!(
                                        "Array index out of bounds: index {} for length {}",
                                        i,
                                        borrowed.len()
                                    ),
                                    span: *idx_span,
                                }));
                            }
                            borrowed[*i as usize] = val;
                        } else if let Value::Map(map) = &arr_val {
                            let k = match idx_val {
                                Value::String(s) => s,
                                other => other.to_string(),
                            };
                            map.lock().unwrap().insert(k, val);
                        } else {
                            return Err(ControlFlow::Error(RuntimeError::General {
                                message: "Cannot assign index on non-array / non-map".to_string(),
                                span: *span,
                            }));
                        }
                    }
                    _ => {
                        return Err(ControlFlow::Error(RuntimeError::General {
                            message: "Invalid assignment target".to_string(),
                            span: *span,
                        }));
                    }
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_val = self.eval_expr(condition).map_err(ControlFlow::Error)?;
                if cond_val.is_truthy() {
                    self.enter_scope();
                    let res = self.execute_block(then_branch);
                    self.exit_scope();
                    res?;
                } else if let Some(else_b) = else_branch {
                    self.enter_scope();
                    let res = self.execute_block(else_b);
                    self.exit_scope();
                    res?;
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                while self
                    .eval_expr(condition)
                    .map_err(ControlFlow::Error)?
                    .is_truthy()
                {
                    self.enter_scope();
                    let res = self.execute_block(body);
                    self.exit_scope();
                    res?;
                }
            }
            Stmt::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                self.enter_scope();
                if let Some(i) = init {
                    self.execute_statement(i)?;
                }
                loop {
                    if let Some(c) = condition {
                        if !self.eval_expr(c).map_err(ControlFlow::Error)?.is_truthy() {
                            break;
                        }
                    }
                    self.enter_scope();
                    let res = self.execute_block(body);
                    self.exit_scope();
                    res?;

                    if let Some(u) = update {
                        self.execute_statement(u)?;
                    }
                }
                self.exit_scope();
            }
            Stmt::ForIn {
                item_name,
                iterable,
                body,
                span,
            } => {
                let iter_val = self.eval_expr(iterable).map_err(ControlFlow::Error)?;
                match iter_val {
                    Value::Array(arr) => {
                        let elements = arr.lock().unwrap().clone();
                        for item in elements {
                            self.enter_scope();
                            if let Some(scope) = self.scopes.last_mut() {
                                scope.insert(item_name.clone(), item);
                            }
                            let res = self.execute_block(body);
                            self.exit_scope();
                            res?;
                        }
                    }
                    Value::Map(map) => {
                        let keys: Vec<String> = map.lock().unwrap().keys().cloned().collect();
                        for k in keys {
                            self.enter_scope();
                            if let Some(scope) = self.scopes.last_mut() {
                                scope.insert(item_name.clone(), Value::String(k));
                            }
                            let res = self.execute_block(body);
                            self.exit_scope();
                            res?;
                        }
                    }
                    Value::String(s) => {
                        for ch in s.chars() {
                            self.enter_scope();
                            if let Some(scope) = self.scopes.last_mut() {
                                scope.insert(item_name.clone(), Value::String(ch.to_string()));
                            }
                            let res = self.execute_block(body);
                            self.exit_scope();
                            res?;
                        }
                    }
                    _ => {
                        return Err(ControlFlow::Error(RuntimeError::General {
                            message: format!(
                                "'for..in' loop requires iterable, found '{iter_val}'"
                            ),
                            span: *span,
                        }));
                    }
                }
            }
            Stmt::Return { value, .. } => {
                let ret_val = if let Some(v) = value {
                    self.eval_expr(v).map_err(ControlFlow::Error)?
                } else {
                    Value::Void
                };
                return Err(ControlFlow::Return(ret_val));
            }
            Stmt::Expr { expr, .. } => {
                self.eval_expr(expr).map_err(ControlFlow::Error)?;
            }
            Stmt::Throw { expr, span } => {
                let val = self.eval_expr(expr).map_err(ControlFlow::Error)?;
                let msg = format!("{val}");
                return Err(ControlFlow::Error(RuntimeError::General {
                    message: msg,
                    span: *span,
                }));
            }
            Stmt::TryCatch {
                try_block,
                catch_param,
                catch_block,
                finally_block,
                ..
            } => {
                self.enter_scope();
                let try_res = self.execute_block(try_block);
                self.exit_scope();

                let mut outcome = match try_res {
                    Ok(()) => Ok(()),
                    Err(ControlFlow::Error(err)) => {
                        if let Some(cb) = catch_block {
                            self.enter_scope();
                            if let Some(p) = catch_param {
                                let msg = match &err {
                                    RuntimeError::General { message, .. } => message.clone(),
                                };
                                self.define_variable(p, Value::String(msg));
                            }
                            let c_res = self.execute_block(cb);
                            self.exit_scope();
                            c_res
                        } else {
                            Err(ControlFlow::Error(err))
                        }
                    }
                    Err(ControlFlow::Return(v)) => Err(ControlFlow::Return(v)),
                };

                // Finally block MUST execute unconditionally
                if let Some(fb) = finally_block {
                    self.enter_scope();
                    let fin_res = self.execute_block(fb);
                    self.exit_scope();
                    if fin_res.is_err() {
                        outcome = fin_res;
                    }
                }

                outcome?;
            }
        }
        Ok(())
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Integer(i, _) => Ok(Value::Int(*i)),
            Expr::Float(f, _) => Ok(Value::Float(*f)),
            Expr::StringLit(s, _) => Ok(Value::String(s.clone())),
            Expr::BoolLit(b, _) => Ok(Value::Bool(*b)),
            Expr::This(span) => {
                if let Some(inst) = &self.current_instance {
                    Ok(Value::Object(inst.clone()))
                } else {
                    Err(RuntimeError::General {
                        message: "'this' used outside of an instance method".to_string(),
                        span: *span,
                    })
                }
            }
            Expr::Variable(name, span) => {
                if let Some(val) = self.get_variable(name) {
                    Ok(val)
                } else if let Some(inst) = &self.current_instance {
                    if let Some(val) = inst.lock().unwrap().fields.get(name) {
                        return Ok(val.clone());
                    }
                    Err(RuntimeError::General {
                        message: format!("Undefined variable or field '{name}'"),
                        span: *span,
                    })
                } else if let Some(func) = self.functions.get(name).cloned() {
                    Ok(Value::Lambda {
                        params: func.params.iter().map(|p| p.name.clone()).collect(),
                        body_expr: None,
                        body_block: Some(func.body.clone()),
                        captured_scopes: vec![],
                    })
                } else if self.classes.contains_key(name) {
                    Ok(Value::String(name.clone()))
                } else {
                    Err(RuntimeError::General {
                        message: format!("Undefined variable '{name}'"),
                        span: *span,
                    })
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let l_val = self.eval_expr(left)?;
                let r_val = self.eval_expr(right)?;

                match op {
                    BinaryOp::Add => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l.wrapping_add(r))),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
                        (l, r) => Ok(Value::String(format!("{l}{r}"))),
                    },
                    BinaryOp::Sub => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l.wrapping_sub(r))),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
                        _ => Err(RuntimeError::General {
                            message: "Cannot subtract non-numeric values".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Mul => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l.wrapping_mul(r))),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
                        _ => Err(RuntimeError::General {
                            message: "Cannot multiply non-numeric values".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Div => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => {
                            if r == 0 {
                                Err(RuntimeError::General {
                                    message: "Division by zero".to_string(),
                                    span: *span,
                                })
                            } else {
                                Ok(Value::Int(l / r))
                            }
                        }
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
                        _ => Err(RuntimeError::General {
                            message: "Cannot divide non-numeric values".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::EqualEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l == r)),
                        (Value::Float(l), Value::Float(r)) => {
                            Ok(Value::Bool((l - r).abs() < f64::EPSILON))
                        }
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l == r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l == r)),
                        _ => Ok(Value::Bool(false)),
                    },
                    BinaryOp::NotEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l != r)),
                        (Value::Float(l), Value::Float(r)) => {
                            Ok(Value::Bool((l - r).abs() >= f64::EPSILON))
                        }
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l != r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l != r)),
                        _ => Ok(Value::Bool(true)),
                    },
                    BinaryOp::Less => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l < r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l < r)),
                        _ => Err(RuntimeError::General {
                            message: "'<' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::LessEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l <= r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l <= r)),
                        _ => Err(RuntimeError::General {
                            message: "'<=' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Greater => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l > r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l > r)),
                        _ => Err(RuntimeError::General {
                            message: "'>' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::GreaterEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l >= r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l >= r)),
                        _ => Err(RuntimeError::General {
                            message: "'>=' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::And => match (l_val, r_val) {
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l && r)),
                        _ => Err(RuntimeError::General {
                            message: "'&&' requires booleans".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Or => match (l_val, r_val) {
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l || r)),
                        _ => Err(RuntimeError::General {
                            message: "'||' requires booleans".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Mod => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => {
                            if r == 0 {
                                Err(RuntimeError::General {
                                    message: "Modulo by zero".to_string(),
                                    span: *span,
                                })
                            } else {
                                Ok(Value::Int(l % r))
                            }
                        }
                        _ => Err(RuntimeError::General {
                            message: "'%' requires integers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::BitwiseAnd => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l & r)),
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l && r)),
                        _ => Err(RuntimeError::General {
                            message: "'&' requires integers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::BitwiseOr => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l | r)),
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l || r)),
                        _ => Err(RuntimeError::General {
                            message: "'|' requires integers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::BitwiseXor => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l ^ r)),
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l ^ r)),
                        _ => Err(RuntimeError::General {
                            message: "'^' requires integers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Shl => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l << (r & 63))),
                        _ => Err(RuntimeError::General {
                            message: "'<<' requires integers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Shr => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => {
                            Ok(Value::Int(((l as u64) >> (r & 63)) as i64))
                        }
                        _ => Err(RuntimeError::General {
                            message: "'>>' requires integers".to_string(),
                            span: *span,
                        }),
                    },
                }
            }
            Expr::Call { callee, args, span } => {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg)?);
                }

                match callee.as_ref() {
                    Expr::Variable(fn_name, _) => {
                        // Check built-in functions
                        if fn_name == "println" {
                            if let Some(first) = evaluated_args.first() {
                                println!("{first}");
                            } else {
                                println!();
                            }
                            return Ok(Value::Void);
                        } else if fn_name == "print" {
                            if let Some(first) = evaluated_args.first() {
                                print!("{first}");
                            }
                            return Ok(Value::Void);
                        } else if fn_name == "readln" {
                            let mut input = String::new();
                            let _ = std::io::stdin().read_line(&mut input);
                            return Ok(Value::String(
                                input.trim_end_matches(['\r', '\n']).to_string(),
                            ));
                        } else if fn_name == "abs" {
                            if let Some(Value::Int(i)) = evaluated_args.first() {
                                return Ok(Value::Int(i.abs()));
                            }
                            return Ok(Value::Int(0));
                        } else if fn_name == "min" {
                            if evaluated_args.len() == 1 {
                                if let Value::Array(arr) = &evaluated_args[0] {
                                    let b = arr.lock().unwrap();
                                    if b.is_empty() {
                                        return Ok(Value::Int(0));
                                    }
                                    let mut m = match &b[0] {
                                        Value::Int(v) => *v,
                                        _ => 0,
                                    };
                                    for elem in &b[1..] {
                                        if let Value::Int(v) = elem {
                                            if *v < m {
                                                m = *v;
                                            }
                                        }
                                    }
                                    return Ok(Value::Int(m));
                                } else if let Value::Int(v) = &evaluated_args[0] {
                                    return Ok(Value::Int(*v));
                                }
                            } else if evaluated_args.len() >= 2 {
                                let mut m = match &evaluated_args[0] {
                                    Value::Int(v) => *v,
                                    _ => 0,
                                };
                                for arg in &evaluated_args[1..] {
                                    if let Value::Int(v) = arg {
                                        if *v < m {
                                            m = *v;
                                        }
                                    }
                                }
                                return Ok(Value::Int(m));
                            }
                            return Ok(Value::Int(0));
                        } else if fn_name == "max" {
                            if evaluated_args.len() == 1 {
                                if let Value::Array(arr) = &evaluated_args[0] {
                                    let b = arr.lock().unwrap();
                                    if b.is_empty() {
                                        return Ok(Value::Int(0));
                                    }
                                    let mut m = match &b[0] {
                                        Value::Int(v) => *v,
                                        _ => 0,
                                    };
                                    for elem in &b[1..] {
                                        if let Value::Int(v) = elem {
                                            if *v > m {
                                                m = *v;
                                            }
                                        }
                                    }
                                    return Ok(Value::Int(m));
                                } else if let Value::Int(v) = &evaluated_args[0] {
                                    return Ok(Value::Int(*v));
                                }
                            } else if evaluated_args.len() >= 2 {
                                let mut m = match &evaluated_args[0] {
                                    Value::Int(v) => *v,
                                    _ => 0,
                                };
                                for arg in &evaluated_args[1..] {
                                    if let Value::Int(v) = arg {
                                        if *v > m {
                                            m = *v;
                                        }
                                    }
                                }
                                return Ok(Value::Int(m));
                            }
                            return Ok(Value::Int(0));
                        }

                        // User-defined function call
                        if let Some(func) = self.functions.get(fn_name).cloned() {
                            match self.call_function(&func, evaluated_args) {
                                Ok(val) => Ok(val),
                                Err(ControlFlow::Return(val)) => Ok(val),
                                Err(ControlFlow::Error(err)) => Err(err),
                            }
                        } else if let Some(val) = self.get_variable(fn_name) {
                            if matches!(val, Value::Lambda { .. }) {
                                return self.invoke_lambda(&val, evaluated_args, *span);
                            }
                            Err(RuntimeError::General {
                                message: format!("'{fn_name}' is not a function"),
                                span: *span,
                            })
                        } else {
                            Err(RuntimeError::General {
                                message: format!("Call to undefined function '{fn_name}'"),
                                span: *span,
                            })
                        }
                    }
                    Expr::MemberAccess { object, member, .. } => {
                        // Check for static Math namespace calls
                        if let Expr::Variable(obj_name, _) = object.as_ref() {
                            if obj_name == "Math" {
                                if member == "min" {
                                    if evaluated_args.len() == 1 {
                                        if let Value::Array(arr) = &evaluated_args[0] {
                                            let b = arr.lock().unwrap();
                                            if b.is_empty() {
                                                return Ok(Value::Int(0));
                                            }
                                            let mut m = match &b[0] {
                                                Value::Int(v) => *v,
                                                _ => 0,
                                            };
                                            for elem in &b[1..] {
                                                if let Value::Int(v) = elem {
                                                    if *v < m {
                                                        m = *v;
                                                    }
                                                }
                                            }
                                            return Ok(Value::Int(m));
                                        } else if let Value::Int(v) = &evaluated_args[0] {
                                            return Ok(Value::Int(*v));
                                        }
                                    } else if evaluated_args.len() >= 2 {
                                        let mut m = match &evaluated_args[0] {
                                            Value::Int(v) => *v,
                                            _ => 0,
                                        };
                                        for arg in &evaluated_args[1..] {
                                            if let Value::Int(v) = arg {
                                                if *v < m {
                                                    m = *v;
                                                }
                                            }
                                        }
                                        return Ok(Value::Int(m));
                                    }
                                    return Ok(Value::Int(0));
                                }
                                if member == "max" {
                                    if evaluated_args.len() == 1 {
                                        if let Value::Array(arr) = &evaluated_args[0] {
                                            let b = arr.lock().unwrap();
                                            if b.is_empty() {
                                                return Ok(Value::Int(0));
                                            }
                                            let mut m = match &b[0] {
                                                Value::Int(v) => *v,
                                                _ => 0,
                                            };
                                            for elem in &b[1..] {
                                                if let Value::Int(v) = elem {
                                                    if *v > m {
                                                        m = *v;
                                                    }
                                                }
                                            }
                                            return Ok(Value::Int(m));
                                        } else if let Value::Int(v) = &evaluated_args[0] {
                                            return Ok(Value::Int(*v));
                                        }
                                    } else if evaluated_args.len() >= 2 {
                                        let mut m = match &evaluated_args[0] {
                                            Value::Int(v) => *v,
                                            _ => 0,
                                        };
                                        for arg in &evaluated_args[1..] {
                                            if let Value::Int(v) = arg {
                                                if *v > m {
                                                    m = *v;
                                                }
                                            }
                                        }
                                        return Ok(Value::Int(m));
                                    }
                                    return Ok(Value::Int(0));
                                }
                                if member == "abs" {
                                    if let Some(Value::Int(v)) = evaluated_args.first() {
                                        return Ok(Value::Int(v.abs()));
                                    }
                                    return Ok(Value::Int(0));
                                }
                                if member == "clamp" {
                                    if evaluated_args.len() >= 3 {
                                        if let (
                                            Value::Int(val),
                                            Value::Int(low),
                                            Value::Int(high),
                                        ) = (
                                            &evaluated_args[0],
                                            &evaluated_args[1],
                                            &evaluated_args[2],
                                        ) {
                                            let c = if *val < *low {
                                                *low
                                            } else if *val > *high {
                                                *high
                                            } else {
                                                *val
                                            };
                                            return Ok(Value::Int(c));
                                        }
                                    }
                                    return Ok(Value::Int(0));
                                }
                                if member == "pow" {
                                    if evaluated_args.len() >= 2 {
                                        if let (Value::Int(base), Value::Int(exp)) =
                                            (&evaluated_args[0], &evaluated_args[1])
                                        {
                                            if *exp >= 0 {
                                                return Ok(Value::Int(base.pow(*exp as u32)));
                                            }
                                        }
                                    }
                                    return Ok(Value::Int(0));
                                }
                                if member == "sqrt" {
                                    if let Some(Value::Int(v)) = evaluated_args.first() {
                                        if *v >= 0 {
                                            let r = (*v as f64).sqrt() as i64;
                                            return Ok(Value::Int(r));
                                        }
                                    }
                                    return Ok(Value::Int(0));
                                }
                                return Err(RuntimeError::General {
                                    message: format!("Unknown method 'Math.{member}'"),
                                    span: *span,
                                });
                            }

                            // Check for static File namespace calls
                            if obj_name == "File" {
                                match member.as_str() {
                                    "readText" => {
                                        let path = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let content =
                                            std::fs::read_to_string(&path).unwrap_or_default();
                                        return Ok(Value::String(content));
                                    }
                                    "writeText" => {
                                        let path = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let content = evaluated_args
                                            .get(1)
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let ok = std::fs::write(&path, content).is_ok();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "appendText" => {
                                        let path = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let content = evaluated_args
                                            .get(1)
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let ok = std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(&path)
                                            .and_then(|mut f| {
                                                std::io::Write::write_all(
                                                    &mut f,
                                                    content.as_bytes(),
                                                )
                                            })
                                            .is_ok();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "exists" => {
                                        let path = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let ok = std::path::Path::new(&path).exists();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "delete" => {
                                        let path = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let ok = std::fs::remove_file(&path).is_ok();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "size" => {
                                        let path = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let sz = std::fs::metadata(&path)
                                            .map(|m| m.len() as i64)
                                            .unwrap_or(-1);
                                        return Ok(Value::Int(sz));
                                    }
                                    "readLines" => {
                                        let path = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let content =
                                            std::fs::read_to_string(&path).unwrap_or_default();
                                        let lines: Vec<Value> = content
                                            .split('\n')
                                            .map(|l| Value::String(l.to_string()))
                                            .collect();
                                        return Ok(Value::Array(Arc::new(Mutex::new(lines))));
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'File.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            if obj_name == "String" && member == "fromCharCode" {
                                let code = match evaluated_args.first() {
                                    Some(Value::Int(i)) => (*i as u8) as char,
                                    _ => ' ',
                                };
                                return Ok(Value::String(code.to_string()));
                            }

                            // Check for static System namespace calls
                            if obj_name == "System" {
                                match member.as_str() {
                                    "args" => {
                                        let arr: Vec<Value> = self
                                            .args
                                            .iter()
                                            .map(|s| Value::String(s.clone()))
                                            .collect();
                                        return Ok(Value::Array(Arc::new(Mutex::new(arr))));
                                    }
                                    "exit" => {
                                        let code = match evaluated_args.first() {
                                            Some(Value::Int(i)) => *i as i32,
                                            _ => 0,
                                        };
                                        std::process::exit(code);
                                    }
                                    "exec" => {
                                        let cmd = match evaluated_args.first() {
                                            Some(Value::String(s)) => s.clone(),
                                            _ => "".to_string(),
                                        };
                                        let status = std::process::Command::new("sh")
                                            .arg("-c")
                                            .arg(&cmd)
                                            .status();
                                        let code = status
                                            .map(|s| s.code().unwrap_or(0) as i64)
                                            .unwrap_or(-1);
                                        return Ok(Value::Int(code));
                                    }
                                    "getEnv" => {
                                        let key = match evaluated_args.first() {
                                            Some(Value::String(s)) => s.clone(),
                                            Some(other) => format!("{other}"),
                                            None => "".to_string(),
                                        };
                                        let val = std::env::var(&key).unwrap_or_default();
                                        return Ok(Value::String(val));
                                    }
                                    "timeMs" | "currentTimeMillis" => {
                                        use std::time::{SystemTime, UNIX_EPOCH};
                                        let ms = match SystemTime::now().duration_since(UNIX_EPOCH)
                                        {
                                            Ok(d) => d.as_millis() as i64,
                                            Err(_) => 0,
                                        };
                                        return Ok(Value::Int(ms));
                                    }
                                    "println" => {
                                        let out: Vec<String> =
                                            evaluated_args.iter().map(|v| v.to_string()).collect();
                                        println!("{}", out.join(" "));
                                        return Ok(Value::Void);
                                    }
                                    "print" => {
                                        let out: Vec<String> =
                                            evaluated_args.iter().map(|v| v.to_string()).collect();
                                        print!("{}", out.join(" "));
                                        use std::io::Write;
                                        let _ = std::io::stdout().flush();
                                        return Ok(Value::Void);
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'System.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check for static Task calls
                            if obj_name == "Task" {
                                match member.as_str() {
                                    "spawn" => {
                                        let fn_val =
                                            evaluated_args.first().cloned().unwrap_or(Value::Void);
                                        let mut sub_interp = self.clone();
                                        let task_args: Vec<Value> =
                                            evaluated_args.iter().skip(1).cloned().collect();
                                        let handle = std::thread::spawn(move || match fn_val {
                                            Value::Lambda { .. } => sub_interp
                                                .invoke_lambda(&fn_val, task_args, Span::default())
                                                .unwrap_or(Value::Void),
                                            _ => Value::Void,
                                        });
                                        return Ok(Value::Task(crate::value::TaskHandle::new(
                                            handle,
                                        )));
                                    }
                                    "sleep" => {
                                        let ms = match evaluated_args.first() {
                                            Some(Value::Int(i)) => *i.max(&0) as u64,
                                            _ => 0,
                                        };
                                        std::thread::sleep(std::time::Duration::from_millis(ms));
                                        return Ok(Value::Void);
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'Task.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check for static Channel calls
                            if obj_name == "Channel" {
                                match member.as_str() {
                                    "select" => {
                                        let channels_arg =
                                            evaluated_args.first().cloned().unwrap_or(Value::Void);
                                        if let Value::Array(ch_list) = channels_arg {
                                            let channels = ch_list.lock().unwrap().clone();
                                            if channels.is_empty() {
                                                return Ok(Value::Int(-1));
                                            }
                                            loop {
                                                for (idx, ch_val) in channels.iter().enumerate() {
                                                    if let Value::Channel(ref ch) = ch_val {
                                                        if !ch.state.is_empty()
                                                            || ch.state.closed.load(
                                                                std::sync::atomic::Ordering::SeqCst,
                                                            )
                                                        {
                                                            return Ok(Value::Int(idx as i64));
                                                        }
                                                    }
                                                }
                                                std::thread::sleep(
                                                    std::time::Duration::from_millis(1),
                                                );
                                            }
                                        }
                                        return Ok(Value::Int(-1));
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'Channel.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check for static TcpListener calls
                            if obj_name == "TcpListener" {
                                match member.as_str() {
                                    "serveReactor" => {
                                        return Ok(Value::Int(0));
                                    }
                                    "listen" => {
                                        let port = match evaluated_args.first() {
                                            Some(Value::Int(p)) => *p as u16,
                                            _ => 8080,
                                        };
                                        let std_listener =
                                            std::net::TcpListener::bind(("127.0.0.1", port))
                                                .or_else(|_| {
                                                    std::net::TcpListener::bind(("0.0.0.0", port))
                                                })
                                                .map_err(|e| RuntimeError::General {
                                                    message: format!(
                                                "TcpListener failed to bind port {port}: {e}"
                                            ),
                                                    span: *span,
                                                })?;
                                        let actual_port = std_listener
                                            .local_addr()
                                            .map(|a| a.port())
                                            .unwrap_or(port);
                                        return Ok(Value::TcpListener(
                                            crate::value::TcpListenerHandle::new(
                                                actual_port,
                                                std_listener,
                                            ),
                                        ));
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!(
                                                "Unknown method 'TcpListener.{member}'"
                                            ),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check for static TcpStream calls
                            if obj_name == "TcpStream" {
                                match member.as_str() {
                                    "connect" => {
                                        let host = evaluated_args
                                            .first()
                                            .map(|v| v.to_string())
                                            .unwrap_or_else(|| "127.0.0.1".to_string());
                                        let port = match evaluated_args.get(1) {
                                            Some(Value::Int(p)) => *p as u16,
                                            _ => 80,
                                        };
                                        let addr = format!("{host}:{port}");
                                        let std_stream = std::net::TcpStream::connect(&addr)
                                            .map_err(|e| RuntimeError::General {
                                                message: format!(
                                                    "TcpStream failed to connect to {addr}: {e}"
                                                ),
                                                span: *span,
                                            })?;
                                        let _ = std_stream.set_nodelay(true);
                                        return Ok(Value::TcpStream(
                                            crate::value::TcpStreamHandle::new(std_stream),
                                        ));
                                    }
                                    "computeWsAccept" => {
                                        let key = evaluated_args
                                            .first()
                                            .map(|v| v.to_string())
                                            .unwrap_or_default();
                                        return Ok(Value::String(
                                            crate::websocket::compute_websocket_accept(&key),
                                        ));
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'TcpStream.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check for static Arena namespace calls (Memory Domain - Specification 02)
                            if obj_name == "Arena" {
                                match member.as_str() {
                                    "create" => {
                                        let cap = evaluated_args
                                            .first()
                                            .and_then(|v| match v {
                                                Value::Int(n) => Some(*n as usize),
                                                _ => None,
                                            })
                                            .unwrap_or(65536);
                                        return Ok(Value::Arena(crate::value::ArenaHandle::new(
                                            cap,
                                        )));
                                    }
                                    "scoped" => {
                                        let callback = evaluated_args.first().ok_or_else(|| {
                                            RuntimeError::General {
                                                message:
                                                    "Arena.scoped requires a callback function"
                                                        .to_string(),
                                                span: *span,
                                            }
                                        })?;
                                        let arena_handle = crate::value::ArenaHandle::new(65536);
                                        let arena_val = Value::Arena(arena_handle.clone());
                                        let res =
                                            self.invoke_lambda(callback, vec![arena_val], *span);
                                        arena_handle.destroy();
                                        return res;
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'Arena.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check for static Json namespace calls
                            if obj_name == "Json" {
                                match member.as_str() {
                                    "parse" => {
                                        let json_str = match evaluated_args.first() {
                                            Some(Value::String(s)) => s.clone(),
                                            Some(other) => other.to_string(),
                                            None => "".to_string(),
                                        };
                                        return Ok(parse_json_value(&json_str));
                                    }
                                    "stringify" => {
                                        let val = evaluated_args.first().unwrap_or(&Value::Void);
                                        return Ok(Value::String(json_stringify(val)));
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'Json.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check for static Schema methods
                            if obj_name == "Schema" {
                                match member.as_str() {
                                    "describe" => {
                                        let target = evaluated_args.first().unwrap_or(&Value::Void);
                                        let class_name = match target {
                                            Value::String(s) => s.clone(),
                                            Value::Object(obj) => {
                                                obj.lock().unwrap().class_name.clone()
                                            }
                                            _ => target.to_string(),
                                        };
                                        return Ok(Value::String(
                                            self.generate_class_schema(&class_name),
                                        ));
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Unknown method 'Schema.{member}'"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            // Check user-defined namespace function calls
                            let ns_fn_name = format!("{obj_name}_{member}");
                            if let Some(func) = self.functions.get(&ns_fn_name).cloned() {
                                return match self.call_function(&func, evaluated_args) {
                                    Ok(val) => Ok(val),
                                    Err(ControlFlow::Return(val)) => Ok(val),
                                    Err(ControlFlow::Error(err)) => Err(err),
                                };
                            }

                            // Check user-defined class static methods
                            if let Some(cls) = self.classes.get(obj_name).cloned() {
                                for m in &cls.members {
                                    if let ClassMember::Method(method) = m {
                                        if method.func.name == *member && method.is_static {
                                            return match self
                                                .call_function(&method.func, evaluated_args)
                                            {
                                                Ok(val) => Ok(val),
                                                Err(ControlFlow::Return(val)) => Ok(val),
                                                Err(ControlFlow::Error(err)) => Err(err),
                                            };
                                        }
                                    }
                                }
                            }
                        }

                        let obj_val = self.eval_expr(object)?;

                        if let Value::String(s) = &obj_val {
                            match member.as_str() {
                                "length" | "len" => return Ok(Value::Int(s.len() as i64)),
                                "isEmpty" => return Ok(Value::Bool(s.is_empty())),
                                "contains" => {
                                    let sub = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    return Ok(Value::Bool(s.contains(&sub)));
                                }
                                "indexOf" => {
                                    let sub = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    let idx = s.find(&sub).map(|i| i as i64).unwrap_or(-1);
                                    return Ok(Value::Int(idx));
                                }
                                "startsWith" => {
                                    let prefix = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    return Ok(Value::Bool(s.starts_with(&prefix)));
                                }
                                "endsWith" => {
                                    let suffix = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    return Ok(Value::Bool(s.ends_with(&suffix)));
                                }
                                "toUpper" | "toUpperCase" => {
                                    return Ok(Value::String(s.to_uppercase()));
                                }
                                "toLower" | "toLowerCase" => {
                                    return Ok(Value::String(s.to_lowercase()));
                                }
                                "trim" => {
                                    return Ok(Value::String(s.trim().to_string()));
                                }
                                "slice" | "substring" => {
                                    let start = match evaluated_args.first() {
                                        Some(Value::Int(i)) => *i.max(&0) as usize,
                                        _ => 0,
                                    };
                                    let end = match evaluated_args.get(1) {
                                        Some(Value::Int(i)) => *i.max(&0) as usize,
                                        _ => s.len(),
                                    };
                                    let clamped_start = start.min(s.len());
                                    let clamped_end = end.min(s.len()).max(clamped_start);
                                    return Ok(Value::String(
                                        s[clamped_start..clamped_end].to_string(),
                                    ));
                                }
                                "charAt" => {
                                    let idx = match evaluated_args.first() {
                                        Some(Value::Int(i)) => *i,
                                        _ => 0,
                                    };
                                    if idx >= 0 && (idx as usize) < s.len() {
                                        let ch = s
                                            .chars()
                                            .nth(idx as usize)
                                            .map(|c| c.to_string())
                                            .unwrap_or_default();
                                        return Ok(Value::String(ch));
                                    }
                                    return Ok(Value::String("".to_string()));
                                }
                                "charCodeAt" => {
                                    let idx = match evaluated_args.first() {
                                        Some(Value::Int(i)) => *i,
                                        _ => 0,
                                    };
                                    if idx >= 0 && (idx as usize) < s.len() {
                                        let b = s.as_bytes()[idx as usize];
                                        return Ok(Value::Int(b as i64));
                                    }
                                    return Ok(Value::Int(-1));
                                }
                                "replace" => {
                                    let target = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    let repl = evaluated_args
                                        .get(1)
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    return Ok(Value::String(s.replace(&target, &repl)));
                                }
                                "split" => {
                                    let delim = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    let pieces: Vec<Value> = if delim.is_empty() {
                                        s.chars().map(|c| Value::String(c.to_string())).collect()
                                    } else {
                                        s.split(&delim)
                                            .map(|p| Value::String(p.to_string()))
                                            .collect()
                                    };
                                    return Ok(Value::Array(Arc::new(Mutex::new(pieces))));
                                }
                                "toInt" => {
                                    let int_val = s.trim().parse::<i64>().unwrap_or(0);
                                    return Ok(Value::Int(int_val));
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!("Method '{member}' not found on string"),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::Array(arr) = &obj_val {
                            match member.as_str() {
                                "push" => {
                                    if let Some(val) = evaluated_args.first() {
                                        arr.lock().unwrap().push(val.clone());
                                    }
                                    return Ok(Value::Int(arr.lock().unwrap().len() as i64));
                                }
                                "length" | "len" => {
                                    return Ok(Value::Int(arr.lock().unwrap().len() as i64))
                                }
                                "isEmpty" => {
                                    return Ok(Value::Bool(arr.lock().unwrap().is_empty()))
                                }
                                "join" => {
                                    let sep = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    let joined = arr
                                        .lock()
                                        .unwrap()
                                        .iter()
                                        .map(|v| v.to_string())
                                        .collect::<Vec<_>>()
                                        .join(&sep);
                                    return Ok(Value::String(joined));
                                }
                                "map" => {
                                    let fn_val =
                                        evaluated_args.first().cloned().unwrap_or(Value::Void);
                                    let elements = arr.lock().unwrap().clone();
                                    let mut mapped = Vec::new();
                                    for elem in elements {
                                        mapped.push(self.invoke_lambda(
                                            &fn_val,
                                            vec![elem],
                                            *span,
                                        )?);
                                    }
                                    return Ok(Value::Array(Arc::new(Mutex::new(mapped))));
                                }
                                "filter" => {
                                    let fn_val =
                                        evaluated_args.first().cloned().unwrap_or(Value::Void);
                                    let elements = arr.lock().unwrap().clone();
                                    let mut filtered = Vec::new();
                                    for elem in elements {
                                        let res =
                                            self.invoke_lambda(&fn_val, vec![elem.clone()], *span)?;
                                        if res.is_truthy() {
                                            filtered.push(elem);
                                        }
                                    }
                                    return Ok(Value::Array(Arc::new(Mutex::new(filtered))));
                                }
                                "reduce" => {
                                    let fn_val =
                                        evaluated_args.first().cloned().unwrap_or(Value::Void);
                                    let mut acc =
                                        evaluated_args.get(1).cloned().unwrap_or(Value::Void);
                                    let elements = arr.lock().unwrap().clone();
                                    for elem in elements {
                                        acc =
                                            self.invoke_lambda(&fn_val, vec![acc, elem], *span)?;
                                    }
                                    return Ok(acc);
                                }
                                "forEach" => {
                                    let fn_val =
                                        evaluated_args.first().cloned().unwrap_or(Value::Void);
                                    let elements = arr.lock().unwrap().clone();
                                    for elem in elements {
                                        self.invoke_lambda(&fn_val, vec![elem], *span)?;
                                    }
                                    return Ok(Value::Void);
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!("Method '{member}' not found on array"),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::Map(map) = &obj_val {
                            match member.as_str() {
                                "set" => {
                                    let k = match evaluated_args.first() {
                                        Some(Value::String(s)) => s.clone(),
                                        Some(other) => other.to_string(),
                                        None => "".to_string(),
                                    };
                                    let v = evaluated_args.get(1).cloned().unwrap_or(Value::Void);
                                    map.lock().unwrap().insert(k, v);
                                    return Ok(Value::Void);
                                }
                                "get" => {
                                    let k = match evaluated_args.first() {
                                        Some(Value::String(s)) => s.clone(),
                                        Some(other) => other.to_string(),
                                        None => "".to_string(),
                                    };
                                    let val =
                                        map.lock().unwrap().get(&k).cloned().unwrap_or(Value::Void);
                                    return Ok(val);
                                }
                                "has" => {
                                    let k = match evaluated_args.first() {
                                        Some(Value::String(s)) => s.clone(),
                                        Some(other) => other.to_string(),
                                        None => "".to_string(),
                                    };
                                    return Ok(Value::Bool(map.lock().unwrap().contains_key(&k)));
                                }
                                "remove" | "delete" => {
                                    let k = match evaluated_args.first() {
                                        Some(Value::String(s)) => s.clone(),
                                        Some(other) => other.to_string(),
                                        None => "".to_string(),
                                    };
                                    let removed = map.lock().unwrap().remove(&k).is_some();
                                    return Ok(Value::Bool(removed));
                                }
                                "len" | "size" => {
                                    return Ok(Value::Int(map.lock().unwrap().len() as i64));
                                }
                                "keys" => {
                                    let keys: Vec<Value> = map
                                        .lock()
                                        .unwrap()
                                        .keys()
                                        .map(|k| Value::String(k.clone()))
                                        .collect();
                                    return Ok(Value::Array(Arc::new(Mutex::new(keys))));
                                }
                                "values" => {
                                    let vals: Vec<Value> =
                                        map.lock().unwrap().values().cloned().collect();
                                    return Ok(Value::Array(Arc::new(Mutex::new(vals))));
                                }
                                "clear" => {
                                    map.lock().unwrap().clear();
                                    return Ok(Value::Void);
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!("Method '{member}' not found on Map"),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::Task(ref t) = obj_val {
                            match member.as_str() {
                                "join" | "wait" | "await" => {
                                    let mut lock = t.handle.lock().unwrap();
                                    if let Some(h) = lock.take() {
                                        match h.join() {
                                            Ok(val) => return Ok(val),
                                            Err(_) => {
                                                return Err(RuntimeError::General {
                                                    message: "Task execution panicked".to_string(),
                                                    span: *span,
                                                });
                                            }
                                        }
                                    } else if let Some(ref shared) = t.shared_result {
                                        let (lock, cvar) = &**shared;
                                        let mut res_guard = lock.lock().unwrap();
                                        while res_guard.is_none() {
                                            res_guard = cvar.wait(res_guard).unwrap();
                                        }
                                        return Ok(res_guard.clone().unwrap());
                                    }
                                    return Ok(Value::Void);
                                }
                                "isDone" | "isFinished" => {
                                    let lock = t.handle.lock().unwrap();
                                    if let Some(ref h) = *lock {
                                        return Ok(Value::Bool(h.is_finished()));
                                    }
                                    if let Some(ref shared) = t.shared_result {
                                        let (l, _) = &**shared;
                                        return Ok(Value::Bool(l.lock().unwrap().is_some()));
                                    }
                                    return Ok(Value::Bool(true));
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!("Method '{member}' not found on Task"),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::Channel(ref ch) = obj_val {
                            match member.as_str() {
                                "send" => {
                                    let val =
                                        evaluated_args.first().cloned().unwrap_or(Value::Void);
                                    ch.state.send(val);
                                    return Ok(Value::Void);
                                }
                                "recv" | "receive" => {
                                    let val = ch.state.recv().unwrap_or(Value::Void);
                                    return Ok(val);
                                }
                                "tryRecv" => {
                                    let val = ch.state.try_recv().unwrap_or(Value::Void);
                                    return Ok(val);
                                }
                                "trySend" => {
                                    let val =
                                        evaluated_args.first().cloned().unwrap_or(Value::Void);
                                    return Ok(Value::Bool(ch.state.try_send(val)));
                                }
                                "close" => {
                                    ch.state.close();
                                    return Ok(Value::Void);
                                }
                                "len" | "size" => {
                                    return Ok(Value::Int(ch.state.len() as i64));
                                }
                                "isEmpty" => {
                                    return Ok(Value::Bool(ch.state.is_empty()));
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!("Method '{member}' not found on Channel"),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::ThreadPool(ref pool) = obj_val {
                            match member.as_str() {
                                "submit" => {
                                    if evaluated_args.is_empty() {
                                        return Err(RuntimeError::General {
                                            message: "ThreadPool.submit requires at least a function argument".to_string(),
                                            span: *span,
                                        });
                                    }
                                    let fn_val = evaluated_args[0].clone();
                                    let task_arg = evaluated_args.get(1).cloned();
                                    let shared: Arc<(Mutex<Option<Value>>, Condvar)> =
                                        Arc::new((Mutex::new(None), Condvar::new()));
                                    let shared_clone = Arc::clone(&shared);

                                    let mut sub_interp = self.clone();
                                    let job = Box::new(move || {
                                        let result = match fn_val {
                                            Value::Lambda { .. } => {
                                                let args =
                                                    task_arg.map(|a| vec![a]).unwrap_or_default();
                                                sub_interp
                                                    .invoke_lambda(&fn_val, args, Span::default())
                                                    .unwrap_or(Value::Void)
                                            }
                                            _ => Value::Void,
                                        };
                                        let (lock, cvar): &(Mutex<Option<Value>>, Condvar) =
                                            &shared_clone;
                                        let mut res_guard = lock.lock().unwrap();
                                        *res_guard = Some(result);
                                        cvar.notify_all();
                                    });

                                    let _ = pool.sender.send(Some(job));
                                    return Ok(Value::Task(crate::value::TaskHandle::new_pool(
                                        shared,
                                    )));
                                }
                                "shutdown" => {
                                    for _ in 0..pool.workers {
                                        let _ = pool.sender.send(None);
                                    }
                                    let mut threads = pool.threads.lock().unwrap();
                                    for h in threads.drain(..) {
                                        let _ = h.join();
                                    }
                                    return Ok(Value::Void);
                                }
                                "workers" => {
                                    return Ok(Value::Int(pool.workers as i64));
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!(
                                            "Method '{member}' not found on ThreadPool"
                                        ),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::TcpListener(ref lh) = obj_val {
                            match member.as_str() {
                                "accept" => {
                                    let lock = lh.listener.lock().unwrap();
                                    if let Some(ref l) = *lock {
                                        let (stream, _) =
                                            l.accept().map_err(|e| RuntimeError::General {
                                                message: format!("TcpListener.accept failed: {e}"),
                                                span: *span,
                                            })?;
                                        let _ = stream.set_nodelay(true);
                                        return Ok(Value::TcpStream(
                                            crate::value::TcpStreamHandle::new(stream),
                                        ));
                                    } else {
                                        return Err(RuntimeError::General {
                                            message: "TcpListener is closed".to_string(),
                                            span: *span,
                                        });
                                    }
                                }
                                "port" => {
                                    return Ok(Value::Int(lh.port as i64));
                                }
                                "close" => {
                                    let mut lock = lh.listener.lock().unwrap();
                                    *lock = None;
                                    return Ok(Value::Void);
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!(
                                            "Method '{member}' not found on TcpListener"
                                        ),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::TcpStream(ref sh) = obj_val {
                            match member.as_str() {
                                "read" => {
                                    let max_len = match evaluated_args.first() {
                                        Some(Value::Int(n)) if *n > 0 => *n as usize,
                                        _ => 65536,
                                    };
                                    let mut lock = sh.stream.lock().unwrap();
                                    if let Some(ref mut stream) = *lock {
                                        use std::io::Read;
                                        let mut buf = vec![0u8; max_len];
                                        match stream.read(&mut buf) {
                                            Ok(n) => {
                                                let s =
                                                    String::from_utf8_lossy(&buf[..n]).to_string();
                                                return Ok(Value::String(s));
                                            }
                                            Err(e) => {
                                                return Err(RuntimeError::General {
                                                    message: format!("TcpStream.read failed: {e}"),
                                                    span: *span,
                                                });
                                            }
                                        }
                                    } else {
                                        return Err(RuntimeError::General {
                                            message: "TcpStream is closed".to_string(),
                                            span: *span,
                                        });
                                    }
                                }
                                "write" => {
                                    let data = evaluated_args
                                        .first()
                                        .map(|v| v.to_string())
                                        .unwrap_or_default();
                                    let mut lock = sh.stream.lock().unwrap();
                                    if let Some(ref mut stream) = *lock {
                                        use std::io::Write;
                                        match stream.write_all(data.as_bytes()) {
                                            Ok(()) => {
                                                let _ = stream.flush();
                                                return Ok(Value::Int(data.len() as i64));
                                            }
                                            Err(e) => {
                                                return Err(RuntimeError::General {
                                                    message: format!("TcpStream.write failed: {e}"),
                                                    span: *span,
                                                });
                                            }
                                        }
                                    } else {
                                        return Err(RuntimeError::General {
                                            message: "TcpStream is closed".to_string(),
                                            span: *span,
                                        });
                                    }
                                }
                                "close" => {
                                    let mut lock = sh.stream.lock().unwrap();
                                    if let Some(ref mut stream) = *lock {
                                        let _ = stream.shutdown(std::net::Shutdown::Both);
                                    }
                                    *lock = None;
                                    return Ok(Value::Void);
                                }
                                "readWsFrame" => {
                                    let mut lock = sh.stream.lock().unwrap();
                                    if let Some(ref mut stream) = *lock {
                                        match crate::websocket::read_websocket_frame(stream) {
                                            Ok(Some(text)) => return Ok(Value::String(text)),
                                            Ok(None) => return Ok(Value::String(String::new())),
                                            Err(e) => {
                                                return Err(RuntimeError::General {
                                                    message: format!(
                                                        "TcpStream.readWsFrame failed: {e}"
                                                    ),
                                                    span: *span,
                                                });
                                            }
                                        }
                                    } else {
                                        return Err(RuntimeError::General {
                                            message: "TcpStream is closed".to_string(),
                                            span: *span,
                                        });
                                    }
                                }
                                "writeWsFrame" => {
                                    let data = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    let mut lock = sh.stream.lock().unwrap();
                                    if let Some(ref mut stream) = *lock {
                                        match crate::websocket::write_websocket_frame(stream, &data)
                                        {
                                            Ok(n) => return Ok(Value::Int(n as i64)),
                                            Err(e) => {
                                                return Err(RuntimeError::General {
                                                    message: format!(
                                                        "TcpStream.writeWsFrame failed: {e}"
                                                    ),
                                                    span: *span,
                                                });
                                            }
                                        }
                                    } else {
                                        return Err(RuntimeError::General {
                                            message: "TcpStream is closed".to_string(),
                                            span: *span,
                                        });
                                    }
                                }
                                "writeWsClientFrame" => {
                                    let data = evaluated_args
                                        .first()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                    let mut lock = sh.stream.lock().unwrap();
                                    if let Some(ref mut stream) = *lock {
                                        match crate::websocket::write_websocket_client_frame(
                                            stream, &data,
                                        ) {
                                            Ok(n) => return Ok(Value::Int(n as i64)),
                                            Err(e) => {
                                                return Err(RuntimeError::General {
                                                    message: format!(
                                                        "TcpStream.writeWsClientFrame failed: {e}"
                                                    ),
                                                    span: *span,
                                                });
                                            }
                                        }
                                    } else {
                                        return Err(RuntimeError::General {
                                            message: "TcpStream is closed".to_string(),
                                            span: *span,
                                        });
                                    }
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!(
                                            "Method '{member}' not found on TcpStream"
                                        ),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::Arena(ref h) = obj_val {
                            match member.as_str() {
                                "alloc" => {
                                    let arg =
                                        evaluated_args.first().cloned().unwrap_or(Value::Void);
                                    return h.alloc(arg).map_err(|e| RuntimeError::General {
                                        message: e,
                                        span: *span,
                                    });
                                }
                                "reset" => {
                                    h.reset();
                                    return Ok(Value::Void);
                                }
                                "destroy" => {
                                    h.destroy();
                                    return Ok(Value::Void);
                                }
                                "allocatedBytes" => {
                                    return Ok(Value::Int(h.allocated_bytes() as i64));
                                }
                                _ => {
                                    return Err(RuntimeError::General {
                                        message: format!("Method '{member}' not found on Arena"),
                                        span: *span,
                                    });
                                }
                            }
                        }

                        if let Value::Object(inst) = obj_val {
                            let cls_name = inst.lock().unwrap().class_name.clone();

                            // Built-in class File instance methods
                            if cls_name == "File" {
                                let path = match inst.lock().unwrap().fields.get("path") {
                                    Some(Value::String(s)) => s.clone(),
                                    _ => String::new(),
                                };
                                match member.as_str() {
                                    "readText" => {
                                        let content =
                                            std::fs::read_to_string(&path).unwrap_or_default();
                                        return Ok(Value::String(content));
                                    }
                                    "writeText" => {
                                        let content = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let ok = std::fs::write(&path, content).is_ok();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "appendText" => {
                                        let content = evaluated_args
                                            .first()
                                            .map(|a| a.to_string())
                                            .unwrap_or_default();
                                        let ok = std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(&path)
                                            .and_then(|mut f| {
                                                std::io::Write::write_all(
                                                    &mut f,
                                                    content.as_bytes(),
                                                )
                                            })
                                            .is_ok();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "exists" => {
                                        let ok = std::path::Path::new(&path).exists();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "delete" => {
                                        let ok = std::fs::remove_file(&path).is_ok();
                                        return Ok(Value::Bool(ok));
                                    }
                                    "size" => {
                                        let sz = std::fs::metadata(&path)
                                            .map(|m| m.len() as i64)
                                            .unwrap_or(-1);
                                        return Ok(Value::Int(sz));
                                    }
                                    "getPath" => {
                                        return Ok(Value::String(path));
                                    }
                                    "readLines" => {
                                        let content =
                                            std::fs::read_to_string(&path).unwrap_or_default();
                                        let lines: Vec<Value> = content
                                            .split('\n')
                                            .map(|l| Value::String(l.to_string()))
                                            .collect();
                                        return Ok(Value::Array(Arc::new(Mutex::new(lines))));
                                    }
                                    _ => {
                                        return Err(RuntimeError::General {
                                            message: format!("Method '{member}' not found on File"),
                                            span: *span,
                                        });
                                    }
                                }
                            }

                            if cls_name == "Route" {
                                if member == "with" {
                                    let mut inst_lock = inst.lock().unwrap();
                                    let m_list = inst_lock
                                        .fields
                                        .entry("middlewaresList".to_string())
                                        .or_insert_with(|| {
                                            Value::Array(Arc::new(Mutex::new(Vec::new())))
                                        });
                                    if let Value::Array(ref arr) = m_list {
                                        let mut arr_lock = arr.lock().unwrap();
                                        for arg in evaluated_args {
                                            if let Value::Array(ref inner) = arg {
                                                for item in inner.lock().unwrap().iter() {
                                                    arr_lock.push(item.clone());
                                                }
                                            } else {
                                                arr_lock.push(arg);
                                            }
                                        }
                                    }
                                    return Ok(Value::Object(inst.clone()));
                                }
                                if member == "response" {
                                    let mut inst_lock = inst.lock().unwrap();
                                    if evaluated_args.len() == 1 {
                                        inst_lock.fields.insert(
                                            "responseStatusCode".to_string(),
                                            Value::Int(200),
                                        );
                                        let schema_name = match &evaluated_args[0] {
                                            Value::String(s) => s.clone(),
                                            Value::Object(o) => {
                                                o.lock().unwrap().class_name.clone()
                                            }
                                            other => other.to_string(),
                                        };
                                        inst_lock.fields.insert(
                                            "responseSchemaName".to_string(),
                                            Value::String(schema_name.clone()),
                                        );
                                        drop(inst_lock);
                                        let schema_json = self.generate_class_schema(&schema_name);
                                        inst.lock().unwrap().fields.insert(
                                            "responseSchemaJson".to_string(),
                                            Value::String(schema_json),
                                        );
                                    } else if evaluated_args.len() == 2 {
                                        let code = match &evaluated_args[0] {
                                            Value::Int(n) => *n,
                                            _ => 200,
                                        };
                                        inst_lock.fields.insert(
                                            "responseStatusCode".to_string(),
                                            Value::Int(code),
                                        );
                                        let desc = evaluated_args[1].to_string();
                                        inst_lock.fields.insert(
                                            "responseDescription".to_string(),
                                            Value::String(desc),
                                        );
                                    } else if evaluated_args.len() >= 3 {
                                        let code = match &evaluated_args[0] {
                                            Value::Int(n) => *n,
                                            _ => 200,
                                        };
                                        inst_lock.fields.insert(
                                            "responseStatusCode".to_string(),
                                            Value::Int(code),
                                        );
                                        let schema_name = match &evaluated_args[1] {
                                            Value::String(s) => s.clone(),
                                            Value::Object(o) => {
                                                o.lock().unwrap().class_name.clone()
                                            }
                                            other => other.to_string(),
                                        };
                                        inst_lock.fields.insert(
                                            "responseSchemaName".to_string(),
                                            Value::String(schema_name.clone()),
                                        );
                                        let desc = evaluated_args[2].to_string();
                                        inst_lock.fields.insert(
                                            "responseDescription".to_string(),
                                            Value::String(desc),
                                        );
                                        drop(inst_lock);
                                        let schema_json = self.generate_class_schema(&schema_name);
                                        inst.lock().unwrap().fields.insert(
                                            "responseSchemaJson".to_string(),
                                            Value::String(schema_json),
                                        );
                                    }
                                    return Ok(Value::Object(inst.clone()));
                                }
                                if member == "request" || member == "body" {
                                    if let Some(target) = evaluated_args.first() {
                                        let schema_name = match target {
                                            Value::String(s) => s.clone(),
                                            Value::Object(o) => {
                                                o.lock().unwrap().class_name.clone()
                                            }
                                            other => other.to_string(),
                                        };
                                        inst.lock().unwrap().fields.insert(
                                            "requestSchemaName".to_string(),
                                            Value::String(schema_name.clone()),
                                        );
                                        let schema_json = self.generate_class_schema(&schema_name);
                                        inst.lock().unwrap().fields.insert(
                                            "requestSchemaJson".to_string(),
                                            Value::String(schema_json),
                                        );
                                    }
                                    return Ok(Value::Object(inst.clone()));
                                }
                                if member == "tag" {
                                    if let Some(val) = evaluated_args.first() {
                                        inst.lock().unwrap().fields.insert(
                                            "routeTag".to_string(),
                                            Value::String(val.to_string()),
                                        );
                                    }
                                    return Ok(Value::Object(inst.clone()));
                                }
                                if member == "summary" {
                                    if let Some(val) = evaluated_args.first() {
                                        inst.lock().unwrap().fields.insert(
                                            "routeSummary".to_string(),
                                            Value::String(val.to_string()),
                                        );
                                    }
                                    return Ok(Value::Object(inst.clone()));
                                }
                                if member == "description" {
                                    if let Some(val) = evaluated_args.first() {
                                        inst.lock().unwrap().fields.insert(
                                            "routeDescription".to_string(),
                                            Value::String(val.to_string()),
                                        );
                                    }
                                    return Ok(Value::Object(inst.clone()));
                                }
                            }

                            if let Some(cls) = self.classes.get(&cls_name).cloned() {
                                for m in &cls.members {
                                    if let ClassMember::Method(method) = m {
                                        if method.func.name == *member {
                                            // Set current instance for 'this'
                                            let prev_instance = self.current_instance.take();
                                            self.current_instance = Some(inst.clone());

                                            let res =
                                                self.call_function(&method.func, evaluated_args);

                                            self.current_instance = prev_instance;

                                            return match res {
                                                Ok(val) => Ok(val),
                                                Err(ControlFlow::Return(val)) => Ok(val),
                                                Err(ControlFlow::Error(err)) => Err(err),
                                            };
                                        }
                                    }
                                }
                                if let Some(val) = inst.lock().unwrap().fields.get(member) {
                                    if matches!(val, Value::Lambda { .. }) {
                                        let val_clone = val.clone();
                                        return self.invoke_lambda(
                                            &val_clone,
                                            evaluated_args,
                                            *span,
                                        );
                                    }
                                }
                                return Err(RuntimeError::General {
                                    message: format!(
                                        "Method '{member}' not found on class '{cls_name}'"
                                    ),
                                    span: *span,
                                });
                            }
                        }
                        Err(RuntimeError::General {
                            message: format!("Cannot invoke method '{member}' on non-object"),
                            span: *span,
                        })
                    }
                    _ => {
                        let callee_val = self.eval_expr(callee)?;
                        if matches!(callee_val, Value::Lambda { .. }) {
                            return self.invoke_lambda(&callee_val, evaluated_args, *span);
                        }
                        Err(RuntimeError::General {
                            message: format!("Cannot call non-callable value '{callee_val}'"),
                            span: *span,
                        })
                    }
                }
            }
            Expr::MemberAccess {
                object,
                member,
                span,
            } => {
                let obj_val = self.eval_expr(object)?;
                if member == "length" || member == "len" || member == "size" {
                    match &obj_val {
                        Value::Array(arr) => {
                            return Ok(Value::Int(arr.lock().unwrap().len() as i64))
                        }
                        Value::String(s) => return Ok(Value::Int(s.len() as i64)),
                        Value::Map(m) => return Ok(Value::Int(m.lock().unwrap().len() as i64)),
                        Value::Channel(ch) => return Ok(Value::Int(ch.state.len() as i64)),
                        _ => {}
                    }
                }
                if let Value::Object(inst) = obj_val {
                    if let Some(val) = inst.lock().unwrap().fields.get(member) {
                        return Ok(val.clone());
                    }
                    Err(RuntimeError::General {
                        message: format!(
                            "Field '{}' not found on instance of '{}'",
                            member,
                            inst.lock().unwrap().class_name
                        ),
                        span: *span,
                    })
                } else {
                    Err(RuntimeError::General {
                        message: format!("Cannot read property '{member}' on non-object"),
                        span: *span,
                    })
                }
            }
            Expr::New {
                class_name,
                args,
                span,
                ..
            } => {
                self.profiler.record_alloc(class_name);
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg)?);
                }

                if class_name == "Map" {
                    return Ok(Value::Map(Arc::new(Mutex::new(HashMap::new()))));
                }

                if class_name == "Channel" {
                    return Ok(Value::Channel(crate::value::ChannelHandle::new()));
                }

                if class_name == "ThreadPool" {
                    let workers = match evaluated_args.first() {
                        Some(Value::Int(i)) => *i.max(&1) as usize,
                        _ => 4,
                    };
                    return Ok(Value::ThreadPool(crate::value::ThreadPoolHandle::new(
                        workers,
                    )));
                }

                if class_name == "Arena" {
                    let cap = match evaluated_args.first() {
                        Some(Value::Int(i)) => *i.max(&1) as usize,
                        _ => 65536,
                    };
                    return Ok(Value::Arena(crate::value::ArenaHandle::new(cap)));
                }

                // Built-in class File instantiation
                if class_name == "File" && !self.classes.contains_key("File") {
                    let path_val = evaluated_args
                        .first()
                        .cloned()
                        .unwrap_or_else(|| Value::String(String::new()));
                    let mut fields = HashMap::new();
                    fields.insert("path".to_string(), path_val);
                    let instance = Arc::new(Mutex::new(Instance {
                        class_name: "File".to_string(),
                        fields,
                    }));
                    return Ok(Value::Object(instance));
                }

                if let Some(cls) = self.classes.get(class_name).cloned() {
                    let instance = Arc::new(Mutex::new(Instance {
                        class_name: class_name.clone(),
                        fields: HashMap::new(),
                    }));

                    // Initialize fields with default values or initializers
                    for m in &cls.members {
                        if let ClassMember::Field(f) = m {
                            let init_val = if let Some(init) = &f.init {
                                self.eval_expr(init)?
                            } else {
                                match f.ty.name.as_str() {
                                    "int" => Value::Int(0),
                                    "float" => Value::Float(0.0),
                                    "bool" => Value::Bool(false),
                                    "string" => Value::String(String::new()),
                                    _ => Value::Void,
                                }
                            };
                            instance
                                .lock()
                                .unwrap()
                                .fields
                                .insert(f.name.clone(), init_val);
                        }
                    }

                    // Run constructor if present
                    for m in &cls.members {
                        if let ClassMember::Constructor(ctor) = m {
                            let prev_inst = self.current_instance.take();
                            self.current_instance = Some(instance.clone());

                            self.enter_scope();
                            for (p, a) in ctor.params.iter().zip(evaluated_args.clone()) {
                                self.define_variable(&p.name, a);
                            }

                            let res = self.execute_block(&ctor.body);
                            self.exit_scope();
                            self.current_instance = prev_inst;

                            if let Err(ControlFlow::Error(err)) = res {
                                return Err(err);
                            }
                            break;
                        }
                    }

                    Ok(Value::Object(instance))
                } else {
                    Err(RuntimeError::General {
                        message: format!("Cannot instantiate undefined class '{class_name}'"),
                        span: *span,
                    })
                }
            }
            Expr::ArrayLit { elements, .. } => {
                let mut evaluated = Vec::new();
                for el in elements {
                    evaluated.push(self.eval_expr(el)?);
                }
                Ok(Value::Array(Arc::new(Mutex::new(evaluated))))
            }
            Expr::Index { array, index, span } => {
                let arr_val = self.eval_expr(array)?;
                let idx_val = self.eval_expr(index)?;
                match (arr_val, idx_val) {
                    (Value::Array(arr), Value::Int(i)) => {
                        let borrowed = arr.lock().unwrap();
                        if i < 0 || (i as usize) >= borrowed.len() {
                            return Err(RuntimeError::General {
                                message: format!(
                                    "Array index out of bounds: index {} for length {}",
                                    i,
                                    borrowed.len()
                                ),
                                span: *span,
                            });
                        }
                        Ok(borrowed[i as usize].clone())
                    }
                    (Value::String(s), Value::Int(i)) => {
                        let chars: Vec<char> = s.chars().collect();
                        if i < 0 || (i as usize) >= chars.len() {
                            return Err(RuntimeError::General {
                                message: format!(
                                    "String index out of bounds: index {} for length {}",
                                    i,
                                    chars.len()
                                ),
                                span: *span,
                            });
                        }
                        Ok(Value::String(chars[i as usize].to_string()))
                    }
                    (Value::Map(map), idx) => {
                        let key = match idx {
                            Value::String(s) => s,
                            other => other.to_string(),
                        };
                        Ok(map
                            .lock()
                            .unwrap()
                            .get(&key)
                            .cloned()
                            .unwrap_or(Value::Void))
                    }
                    _ => Err(RuntimeError::General {
                        message: "Invalid index operation".to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::Lambda {
                params,
                body_expr,
                body_block,
                span: _,
            } => {
                let param_names = params.iter().map(|p| p.name.clone()).collect();
                Ok(Value::Lambda {
                    params: param_names,
                    body_expr: body_expr.clone(),
                    body_block: body_block.clone(),
                    captured_scopes: self.scopes.clone(),
                })
            }
            Expr::Await { expr, span } => {
                let val = self.eval_expr(expr)?;
                match val {
                    Value::Task(ref t) => {
                        let res = t.await_result().map_err(|msg| RuntimeError::General {
                            message: msg,
                            span: *span,
                        })?;
                        Ok(res)
                    }
                    other => Ok(other),
                }
            }
        }
    }

    /// Evaluates an interactive REPL line, persisting variables and declarations.
    pub fn eval_line(&mut self, line: &str) -> Result<Option<Value>, RuntimeError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        // 1. Try parsing as top-level program items (classes, interfaces, functions, namespaces)
        if let Ok(program) = syntra_syntax::parse(trimmed) {
            if !program.items.is_empty() {
                for item in &program.items {
                    self.register_item(item);
                }
                return Ok(None);
            }
        }

        // 2. Try parsing as statements/expressions inside a synthetic function
        let wrapped = if trimmed.ends_with('}') {
            format!("func __repl_main(): void {{\n{trimmed}\n}}")
        } else {
            format!(
                "func __repl_main(): void {{\n{};\n}}",
                trimmed.trim_end_matches(';')
            )
        };

        let program = syntra_syntax::parse(&wrapped).map_err(|diags| {
            let msg = diags
                .first()
                .map(|d| d.message.clone())
                .unwrap_or_else(|| "Syntax error".to_string());
            RuntimeError::General {
                message: msg,
                span: Span::default(),
            }
        })?;

        if let Some(Item::Function(f)) = program.items.first() {
            let stmts = &f.body.statements;
            let len = stmts.len();
            for (idx, stmt) in stmts.iter().enumerate() {
                let is_last = idx + 1 == len;
                if is_last {
                    if let Stmt::Expr { expr, .. } = stmt {
                        let val = self.eval_expr(expr)?;
                        return Ok(Some(val));
                    }
                }
                match self.execute_statement(stmt) {
                    Ok(()) => {}
                    Err(ControlFlow::Return(v)) => return Ok(Some(v)),
                    Err(ControlFlow::Error(e)) => return Err(e),
                }
            }
        }

        Ok(None)
    }
}

pub fn json_stringify(val: &Value) -> String {
    match val {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 {
                format!("{f:.1}")
            } else {
                f.to_string()
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::String(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    other => out.push(other),
                }
            }
            out.push('"');
            out
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.lock().unwrap().iter().map(json_stringify).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Map(m) => {
            let mut pairs: Vec<String> = m
                .lock()
                .unwrap()
                .iter()
                .map(|(k, v)| {
                    let mut k_esc = String::from("\"");
                    for c in k.chars() {
                        if c == '"' {
                            k_esc.push_str("\\\"");
                        } else {
                            k_esc.push(c);
                        }
                    }
                    k_esc.push('"');
                    format!("{}: {}", k_esc, json_stringify(v))
                })
                .collect();
            pairs.sort();
            format!("{{{}}}", pairs.join(", "))
        }
        Value::Object(obj) => {
            let inst = obj.lock().unwrap();
            let mut pairs: Vec<String> = inst
                .fields
                .iter()
                .map(|(k, v)| {
                    let mut k_esc = String::from("\"");
                    for c in k.chars() {
                        if c == '"' {
                            k_esc.push_str("\\\"");
                        } else {
                            k_esc.push(c);
                        }
                    }
                    k_esc.push('"');
                    format!("{}: {}", k_esc, json_stringify(v))
                })
                .collect();
            pairs.sort();
            format!("{{{}}}", pairs.join(", "))
        }
        Value::Task(t) => format!("\"<Task #{}>\"", t.id),
        Value::Channel(c) => format!("\"<Channel #{}>\"", c.id),
        Value::ThreadPool(p) => format!("\"<ThreadPool #{}>\"", p.id),
        Value::TcpListener(l) => format!("\"<TcpListener port={}>\"", l.port),
        Value::TcpStream(s) => format!("\"<TcpStream #{}>\"", s.id),
        Value::Arena(a) => format!("\"<Arena #{}>\"", a.id),
        Value::Lambda { .. } => "\"<lambda>\"".to_string(),
        Value::Void => "null".to_string(),
    }
}

struct JsonParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Value {
        self.skip_whitespace();
        match self.chars.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('n') => self.parse_null(),
            Some(&c) if c.is_ascii_digit() || c == '-' => self.parse_number(),
            _ => Value::Void,
        }
    }

    fn parse_object(&mut self) -> Value {
        self.chars.next(); // consume '{'
        let mut map = HashMap::new();
        loop {
            self.skip_whitespace();
            if let Some(&'}') = self.chars.peek() {
                self.chars.next();
                break;
            }
            let key = match self.parse_value() {
                Value::String(s) => s,
                other => other.to_string(),
            };
            self.skip_whitespace();
            if let Some(&':') = self.chars.peek() {
                self.chars.next();
            }
            let val = self.parse_value();
            map.insert(key, val);
            self.skip_whitespace();
            if let Some(&',') = self.chars.peek() {
                self.chars.next();
            } else if let Some(&'}') = self.chars.peek() {
                self.chars.next();
                break;
            } else {
                break;
            }
        }
        Value::Map(Arc::new(Mutex::new(map)))
    }

    fn parse_array(&mut self) -> Value {
        self.chars.next(); // consume '['
        let mut arr = Vec::new();
        loop {
            self.skip_whitespace();
            if let Some(&']') = self.chars.peek() {
                self.chars.next();
                break;
            }
            let val = self.parse_value();
            arr.push(val);
            self.skip_whitespace();
            if let Some(&',') = self.chars.peek() {
                self.chars.next();
            } else if let Some(&']') = self.chars.peek() {
                self.chars.next();
                break;
            } else {
                break;
            }
        }
        Value::Array(Arc::new(Mutex::new(arr)))
    }

    fn parse_string(&mut self) -> Value {
        self.chars.next(); // consume '"'
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            if c == '"' {
                break;
            } else if c == '\\' {
                if let Some(esc) = self.chars.next() {
                    match esc {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        other => s.push(other),
                    }
                }
            } else {
                s.push(c);
            }
        }
        Value::String(s)
    }

    fn parse_bool(&mut self) -> Value {
        let mut word = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                word.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        Value::Bool(word == "true")
    }

    fn parse_null(&mut self) -> Value {
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                self.chars.next();
            } else {
                break;
            }
        }
        Value::Void
    }

    fn parse_number(&mut self) -> Value {
        let mut num_str = String::new();
        let mut is_float = false;
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '-' || c == '+' {
                num_str.push(c);
                self.chars.next();
            } else if c == '.' {
                is_float = true;
                num_str.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        if is_float {
            Value::Float(num_str.parse::<f64>().unwrap_or(0.0))
        } else {
            Value::Int(num_str.parse::<i64>().unwrap_or(0))
        }
    }
}

pub fn parse_json_value(input: &str) -> Value {
    let mut parser = JsonParser::new(input);
    parser.parse_value()
}
