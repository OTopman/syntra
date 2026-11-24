use crate::ir::*;
use std::collections::HashMap;
use syntra_frontend::Type;
use syntra_syntax::ast::*;

pub fn build_class_schema(cls: &ClassDecl) -> String {
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
                            custom => format!("{{\"$ref\":\"#/components/schemas/{custom}\"}}"),
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

#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub name: String,
    pub offset: usize,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct ClassLayout {
    pub name: String,
    pub fields: HashMap<String, FieldLayout>,
    pub total_size: usize,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct TryContext {
    catch_block: BlockId,
    finally_block: Option<BlockId>,
}

pub struct Lowerer {
    next_val: usize,
    next_block: usize,
    current_instructions: Vec<Instruction>,
    blocks: Vec<BasicBlock>,
    variables: HashMap<String, ValueId>,
    variable_types: HashMap<String, Type>,
    parent_variable_types: HashMap<String, Type>,
    value_types: HashMap<ValueId, Type>,
    class_layouts: HashMap<String, ClassLayout>,
    class_generic_params: HashMap<String, Vec<String>>,
    method_return_types: HashMap<(String, String), Type>,
    function_return_types: HashMap<String, Type>,
    current_class: Option<String>,
    current_block_id: BlockId,
    try_stack: Vec<TryContext>,
    pub extra_functions: Vec<AirFunction>,
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
    }
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            next_val: 0,
            next_block: 0,
            current_instructions: Vec::new(),
            blocks: Vec::new(),
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            parent_variable_types: HashMap::new(),
            value_types: HashMap::new(),
            class_layouts: HashMap::new(),
            class_generic_params: HashMap::new(),
            method_return_types: HashMap::new(),
            function_return_types: HashMap::new(),
            current_class: None,
            current_block_id: BlockId(0),
            try_stack: Vec::new(),
            extra_functions: Vec::new(),
        }
    }

    fn alloc_val(&mut self) -> ValueId {
        let id = ValueId(self.next_val);
        self.next_val += 1;
        id
    }

    fn alloc_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    pub fn lower_program(&mut self, program: &Program) -> AirModule {
        // Step 1: Pre-compute class layouts (field offsets and sizes)
        for item in &program.items {
            if let Item::Class(cls) = item {
                let mut offset = 0;
                let mut fields = HashMap::new();
                for member in &cls.members {
                    if let ClassMember::Field(f) = member {
                        let ty = Type::from_ast(&f.ty);
                        fields.insert(
                            f.name.clone(),
                            FieldLayout {
                                name: f.name.clone(),
                                offset,
                                ty,
                            },
                        );
                        offset += 8; // 8 bytes per 64-bit slot
                    }
                }
                let total_size = if offset == 0 { 8 } else { offset };
                self.class_layouts.insert(
                    cls.name.clone(),
                    ClassLayout {
                        name: cls.name.clone(),
                        fields,
                        total_size,
                    },
                );

                self.class_generic_params
                    .insert(cls.name.clone(), cls.generic_params.clone());
                for member in &cls.members {
                    if let ClassMember::Method(m) = member {
                        let ret_ty = if m.func.is_async {
                            Type::Class("Task".to_string())
                        } else {
                            m.func
                                .return_type
                                .as_ref()
                                .map(Type::from_ast)
                                .unwrap_or(Type::Void)
                        };
                        self.method_return_types
                            .insert((cls.name.clone(), m.func.name.clone()), ret_ty.clone());
                        self.function_return_types
                            .insert(format!("{}_{}", cls.name, m.func.name), ret_ty);
                    }
                }
            } else if let Item::Function(func) = item {
                let ret_ty = if func.is_async {
                    Type::Class("Task".to_string())
                } else {
                    func.return_type
                        .as_ref()
                        .map(Type::from_ast)
                        .unwrap_or(Type::Void)
                };
                self.function_return_types.insert(func.name.clone(), ret_ty);
            } else if let Item::Namespace(ns) = item {
                for sub in &ns.items {
                    if let Item::Function(func) = sub {
                        let ret_ty = if func.is_async {
                            Type::Class("Task".to_string())
                        } else {
                            func.return_type
                                .as_ref()
                                .map(Type::from_ast)
                                .unwrap_or(Type::Void)
                        };
                        self.function_return_types
                            .insert(format!("{}_{}", ns.name, func.name), ret_ty);
                    }
                }
            }
        }

        // Register built-in File class layout and method return types
        if !self.class_layouts.contains_key("File") {
            let mut fields = HashMap::new();
            fields.insert(
                "path".to_string(),
                FieldLayout {
                    name: "path".to_string(),
                    offset: 0,
                    ty: Type::String,
                },
            );
            self.class_layouts.insert(
                "File".to_string(),
                ClassLayout {
                    name: "File".to_string(),
                    fields,
                    total_size: 8,
                },
            );
            self.method_return_types
                .insert(("File".to_string(), "readText".to_string()), Type::String);
            self.method_return_types
                .insert(("File".to_string(), "writeText".to_string()), Type::Bool);
            self.method_return_types
                .insert(("File".to_string(), "appendText".to_string()), Type::Bool);
            self.method_return_types
                .insert(("File".to_string(), "exists".to_string()), Type::Bool);
            self.method_return_types
                .insert(("File".to_string(), "delete".to_string()), Type::Bool);
            self.method_return_types
                .insert(("File".to_string(), "size".to_string()), Type::Int);
            self.method_return_types
                .insert(("File".to_string(), "getPath".to_string()), Type::String);
            self.method_return_types.insert(
                ("File".to_string(), "readLines".to_string()),
                Type::Array(Box::new(Type::String)),
            );
        }

        let mut functions = Vec::new();

        // Step 2: Lower all items
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    if func.is_async {
                        let (worker, wrapper) = self.lower_async_function(func);
                        functions.push(worker);
                        functions.push(wrapper);
                    } else {
                        functions.push(self.lower_function(func));
                    }
                }
                Item::Namespace(ns) => {
                    for sub in &ns.items {
                        if let Item::Function(func) = sub {
                            let mut namespaced_func = func.clone();
                            namespaced_func.name = format!("{}_{}", ns.name, func.name);
                            if namespaced_func.is_async {
                                let (worker, wrapper) = self.lower_async_function(&namespaced_func);
                                functions.push(worker);
                                functions.push(wrapper);
                            } else {
                                functions.push(self.lower_function(&namespaced_func));
                            }
                        }
                    }
                }
                Item::Import(_) => {}
                Item::Class(cls) => {
                    self.current_class = Some(cls.name.clone());

                    // Lower constructor
                    let mut has_explicit_constructor = false;
                    for member in &cls.members {
                        if let ClassMember::Constructor(ctor) = member {
                            functions.push(self.lower_constructor(cls, ctor));
                            has_explicit_constructor = true;
                            break;
                        }
                    }

                    if !has_explicit_constructor {
                        functions.push(self.lower_default_constructor(cls));
                    }

                    // Lower methods
                    for member in &cls.members {
                        if let ClassMember::Method(m) = member {
                            let mut method_func = m.func.clone();
                            method_func.name = format!("{}_{}", cls.name, m.func.name);
                            if !m.is_static {
                                method_func.params.insert(
                                    0,
                                    Param {
                                        name: "this".to_string(),
                                        ty: TypeRef {
                                            name: cls.name.clone(),
                                            generic_args: Vec::new(),
                                            is_array: false,
                                            span: m.func.span,
                                        },
                                        span: m.func.span,
                                    },
                                );
                            }
                            if method_func.is_async {
                                let (worker, wrapper) = self.lower_async_function(&method_func);
                                functions.push(worker);
                                functions.push(wrapper);
                            } else {
                                functions.push(self.lower_function(&method_func));
                            }
                        }
                    }

                    self.current_class = None;
                }
                Item::Interface(_) => {}
            }
        }

        functions.append(&mut self.extra_functions);

        // Pre-register class schemas in main function
        let mut class_schemas = Vec::new();
        for item in &program.items {
            if let Item::Class(cls) = item {
                let schema_json = build_class_schema(cls);
                class_schemas.push((cls.name.clone(), schema_json, cls.span));
            }
        }

        if !class_schemas.is_empty() {
            for func in &mut functions {
                if func.name == "main" || func.name == "syn_fn_main" {
                    if let Some(entry_block) =
                        func.blocks.iter_mut().find(|b| b.id == func.entry_block)
                    {
                        let mut reg_insts = Vec::new();
                        for (c_name, s_json, span) in &class_schemas {
                            let name_val = ValueId(self.next_val);
                            self.next_val += 1;
                            reg_insts.push(Instruction {
                                result: name_val,
                                ty: Type::String,
                                op: Op::ConstString(c_name.clone()),
                                span: *span,
                            });
                            let json_val = ValueId(self.next_val);
                            self.next_val += 1;
                            reg_insts.push(Instruction {
                                result: json_val,
                                ty: Type::String,
                                op: Op::ConstString(s_json.clone()),
                                span: *span,
                            });
                            let dummy = ValueId(self.next_val);
                            self.next_val += 1;
                            reg_insts.push(Instruction {
                                result: dummy,
                                ty: Type::Void,
                                op: Op::Call {
                                    func_name: "syntra_schema_register".to_string(),
                                    args: vec![name_val, json_val],
                                },
                                span: *span,
                            });
                        }
                        reg_insts.append(&mut entry_block.instructions);
                        entry_block.instructions = reg_insts;
                    }
                    break;
                }
            }
        }

        AirModule {
            name: "main_module".to_string(),
            functions,
            class_sizes: self
                .class_layouts
                .iter()
                .map(|(k, v)| (k.clone(), v.total_size))
                .collect(),
        }
    }

    fn lower_constructor(&mut self, cls: &ClassDecl, ctor: &ConstructorDecl) -> AirFunction {
        self.blocks.clear();
        self.current_instructions.clear();
        self.variables.clear();
        self.variable_types.clear();
        self.value_types.clear();

        let entry_id = self.alloc_block();
        self.current_block_id = entry_id;

        let mut params = Vec::new();
        for p in &ctor.params {
            let v_id = self.alloc_val();
            let ty = Type::from_ast(&p.ty);
            params.push(AirParam {
                id: v_id,
                name: p.name.clone(),
                ty: ty.clone(),
            });

            let slot = self.alloc_val();
            self.current_instructions.push(Instruction {
                result: slot,
                ty: ty.clone(),
                op: Op::Alloc {
                    domain: MemoryDomain::Stack,
                    ty: ty.clone(),
                },
                span: p.span,
            });
            self.value_types.insert(slot, ty.clone());

            let store_res = self.alloc_val();
            self.current_instructions.push(Instruction {
                result: store_res,
                ty: Type::Void,
                op: Op::Store {
                    ptr: slot,
                    val: v_id,
                },
                span: p.span,
            });
            self.value_types.insert(store_res, Type::Void);

            self.variables.insert(p.name.clone(), slot);
            self.variable_types.insert(p.name.clone(), ty);
        }

        // Allocate 'this' instance
        let this_id = self.alloc_val();
        let class_type = Type::Class(cls.name.clone());
        self.current_instructions.push(Instruction {
            result: this_id,
            ty: class_type.clone(),
            op: Op::Alloc {
                domain: MemoryDomain::Arena,
                ty: class_type.clone(),
            },
            span: ctor.span,
        });
        self.variables.insert("this".to_string(), this_id);
        self.variable_types
            .insert("this".to_string(), class_type.clone());
        self.value_types.insert(this_id, class_type.clone());

        // Lower constructor body
        let terminated = self.lower_block_stmts(&ctor.body);
        if !terminated {
            let insts = std::mem::take(&mut self.current_instructions);
            self.blocks.push(BasicBlock {
                id: self.current_block_id,
                params: Vec::new(),
                instructions: insts,
                terminator: Terminator::Return(Some(this_id), ctor.span),
            });
        }

        AirFunction {
            name: format!("{}_new", cls.name),
            params,
            return_type: class_type,
            blocks: std::mem::take(&mut self.blocks),
            entry_block: entry_id,
        }
    }

    fn lower_default_constructor(&mut self, cls: &ClassDecl) -> AirFunction {
        self.blocks.clear();
        self.current_instructions.clear();
        self.variables.clear();
        self.variable_types.clear();
        self.value_types.clear();

        let entry_id = self.alloc_block();
        let this_id = self.alloc_val();
        let class_type = Type::Class(cls.name.clone());

        self.current_instructions.push(Instruction {
            result: this_id,
            ty: class_type.clone(),
            op: Op::Alloc {
                domain: MemoryDomain::Arena,
                ty: class_type.clone(),
            },
            span: cls.span,
        });

        let insts = std::mem::take(&mut self.current_instructions);
        self.blocks.push(BasicBlock {
            id: entry_id,
            params: Vec::new(),
            instructions: insts,
            terminator: Terminator::Return(Some(this_id), cls.span),
        });

        AirFunction {
            name: format!("{}_new", cls.name),
            params: Vec::new(),
            return_type: class_type,
            blocks: std::mem::take(&mut self.blocks),
            entry_block: entry_id,
        }
    }

    fn lower_function(&mut self, func: &FunctionDecl) -> AirFunction {
        self.blocks.clear();
        self.current_instructions.clear();
        self.variables.clear();
        self.variable_types.clear();
        self.value_types.clear();

        let entry_id = self.alloc_block();
        self.current_block_id = entry_id;

        let mut params = Vec::new();
        for p in &func.params {
            let v_id = self.alloc_val();
            let ty = Type::from_ast(&p.ty);
            params.push(AirParam {
                id: v_id,
                name: p.name.clone(),
                ty: ty.clone(),
            });

            if p.name == "this" {
                self.variables.insert(p.name.clone(), v_id);
                self.variable_types.insert(p.name.clone(), ty.clone());
                self.value_types.insert(v_id, ty);
            } else {
                let slot = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: slot,
                    ty: ty.clone(),
                    op: Op::Alloc {
                        domain: MemoryDomain::Stack,
                        ty: ty.clone(),
                    },
                    span: p.span,
                });
                self.value_types.insert(slot, ty.clone());

                let store_res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: store_res,
                    ty: Type::Void,
                    op: Op::Store {
                        ptr: slot,
                        val: v_id,
                    },
                    span: p.span,
                });
                self.value_types.insert(store_res, Type::Void);

                self.variables.insert(p.name.clone(), slot);
                self.variable_types.insert(p.name.clone(), ty);
            }
        }

        let return_type = func
            .return_type
            .as_ref()
            .map(Type::from_ast)
            .unwrap_or(Type::Void);

        // Lower body
        let terminated = self.lower_block_stmts(&func.body);
        if !terminated {
            let insts = std::mem::take(&mut self.current_instructions);
            self.blocks.push(BasicBlock {
                id: self.current_block_id,
                params: Vec::new(),
                instructions: insts,
                terminator: Terminator::Return(None, func.span),
            });
        }

        AirFunction {
            name: func.name.clone(),
            params,
            return_type,
            blocks: std::mem::take(&mut self.blocks),
            entry_block: entry_id,
        }
    }

    fn lower_async_function(&mut self, func: &FunctionDecl) -> (AirFunction, AirFunction) {
        let worker_name = format!("__async_worker_{}", func.name);
        let mut worker_decl = func.clone();
        worker_decl.name = worker_name.clone();
        worker_decl.is_async = false;
        if worker_decl.params.is_empty() {
            worker_decl.params.push(Param {
                name: "_unused".to_string(),
                ty: TypeRef {
                    name: "int".to_string(),
                    generic_args: vec![],
                    is_array: false,
                    span: func.span,
                },
                span: func.span,
            });
        }
        let worker_air = self.lower_function(&worker_decl);

        self.blocks.clear();
        self.current_instructions.clear();
        self.variables.clear();
        self.variable_types.clear();
        self.value_types.clear();

        let entry_id = self.alloc_block();
        self.current_block_id = entry_id;

        let mut wrapper_params = Vec::new();
        let mut arg_vals = Vec::new();
        for p in &func.params {
            let v_id = self.alloc_val();
            let ty = Type::from_ast(&p.ty);
            wrapper_params.push(AirParam {
                id: v_id,
                name: p.name.clone(),
                ty: ty.clone(),
            });
            self.value_types.insert(v_id, ty);
            arg_vals.push(v_id);
        }

        let fn_ptr = self.alloc_val();
        self.current_instructions.push(Instruction {
            result: fn_ptr,
            ty: Type::Int,
            op: Op::FuncAddr {
                func_name: worker_name,
            },
            span: func.span,
        });
        self.value_types.insert(fn_ptr, Type::Int);

        let argc_val = self.alloc_val();
        self.current_instructions.push(Instruction {
            result: argc_val,
            ty: Type::Int,
            op: Op::ConstInt(arg_vals.len() as i64),
            span: func.span,
        });
        self.value_types.insert(argc_val, Type::Int);

        let mut spawn_args = vec![fn_ptr, argc_val];
        for i in 0..6 {
            if i < arg_vals.len() {
                spawn_args.push(arg_vals[i]);
            } else {
                let zero = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: zero,
                    ty: Type::Int,
                    op: Op::ConstInt(0),
                    span: func.span,
                });
                self.value_types.insert(zero, Type::Int);
                spawn_args.push(zero);
            }
        }

        let task_val = self.alloc_val();
        let task_ty = Type::Class("Task".to_string());
        self.current_instructions.push(Instruction {
            result: task_val,
            ty: task_ty.clone(),
            op: Op::Call {
                func_name: "syntra_task_spawn_n".to_string(),
                args: spawn_args,
            },
            span: func.span,
        });
        self.value_types.insert(task_val, task_ty.clone());

        let insts = std::mem::take(&mut self.current_instructions);
        self.blocks.push(BasicBlock {
            id: entry_id,
            params: Vec::new(),
            instructions: insts,
            terminator: Terminator::Return(Some(task_val), func.span),
        });

        let wrapper_air = AirFunction {
            name: func.name.clone(),
            params: wrapper_params,
            return_type: task_ty,
            blocks: std::mem::take(&mut self.blocks),
            entry_block: entry_id,
        };

        (worker_air, wrapper_air)
    }

    /// Returns true if the block ended with a terminator (Return)
    fn lower_block_stmts(&mut self, block: &Block) -> bool {
        for stmt in &block.statements {
            match stmt {
                Stmt::VarDecl {
                    name,
                    ty,
                    init,
                    span,
                    ..
                } => {
                    let val = self.lower_expression(init);
                    let var_ty = Type::from_ast(ty);

                    let slot = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: slot,
                        ty: var_ty.clone(),
                        op: Op::Alloc {
                            domain: MemoryDomain::Stack,
                            ty: var_ty.clone(),
                        },
                        span: *span,
                    });
                    self.value_types.insert(slot, var_ty.clone());

                    let store_res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: store_res,
                        ty: Type::Void,
                        op: Op::Store { ptr: slot, val },
                        span: *span,
                    });
                    self.value_types.insert(store_res, Type::Void);

                    self.variables.insert(name.clone(), slot);
                    self.variable_types.insert(name.clone(), var_ty);
                }
                Stmt::Assignment {
                    target,
                    value,
                    span,
                } => {
                    let val = self.lower_expression(value);
                    match target {
                        Expr::Variable(name, _) => {
                            if let Some(slot) = self.variables.get(name).copied() {
                                let store_res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: store_res,
                                    ty: Type::Void,
                                    op: Op::Store { ptr: slot, val },
                                    span: *span,
                                });
                                self.value_types.insert(store_res, Type::Void);
                            }
                        }
                        Expr::MemberAccess { object, member, .. } => {
                            let obj_val = self.lower_expression(object);
                            let obj_cls = match self.value_types.get(&obj_val) {
                                Some(Type::Class(cls)) => Some(cls.clone()),
                                Some(Type::GenericInstance(cls, _)) => Some(cls.clone()),
                                _ => self.get_object_class(object),
                            };
                            if let Some(cls_name) = obj_cls {
                                let field_info = self
                                    .class_layouts
                                    .get(&cls_name)
                                    .and_then(|l| l.fields.get(member))
                                    .map(|f| f.offset);

                                if let Some(offset) = field_info {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::SetField {
                                            object: obj_val,
                                            field_name: member.clone(),
                                            offset,
                                            val,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                }
                            }
                        }
                        Expr::Index { array, index, .. } => {
                            let arr_val = self.lower_expression(array);
                            let idx_val = self.lower_expression(index);
                            let is_map = match self.value_types.get(&arr_val) {
                                Some(Type::Class(c)) if c == "Map" => true,
                                Some(Type::GenericInstance(c, _)) if c == "Map" => true,
                                _ => {
                                    if let Expr::Variable(arr_name, _) = array.as_ref() {
                                        matches!(self.variable_types.get(arr_name), Some(Type::Class(c)) | Some(Type::GenericInstance(c, _)) if c == "Map")
                                    } else {
                                        false
                                    }
                                }
                            };
                            if is_map {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::Void,
                                    op: Op::Call {
                                        func_name: "syntra_map_set".to_string(),
                                        args: vec![arr_val, idx_val, val],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::Void);
                            } else {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::Void,
                                    op: Op::SetIndex {
                                        array: arr_val,
                                        index: idx_val,
                                        val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::Void);
                            }
                        }
                        _ => {}
                    }
                }
                Stmt::Return { value, span } => {
                    let val = value.as_ref().map(|v| self.lower_expression(v));
                    let insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: self.current_block_id,
                        params: Vec::new(),
                        instructions: insts,
                        terminator: Terminator::Return(val, *span),
                    });
                    return true;
                }
                Stmt::Expr { expr, .. } => {
                    self.lower_expression(expr);
                }
                Stmt::If {
                    condition,
                    then_branch,
                    else_branch,
                    span,
                } => {
                    let cond_val = self.lower_expression(condition);
                    let then_bb = self.alloc_block();
                    let else_bb = self.alloc_block();
                    let merge_bb = self.alloc_block();

                    let has_else = else_branch.is_some();
                    let else_target = if has_else { else_bb } else { merge_bb };

                    // Current block -> CondBranch
                    let insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: self.current_block_id,
                        params: Vec::new(),
                        instructions: insts,
                        terminator: Terminator::CondBranch {
                            cond: cond_val,
                            then_block: then_bb,
                            then_args: Vec::new(),
                            else_block: else_target,
                            else_args: Vec::new(),
                            span: *span,
                        },
                    });

                    // Then block
                    self.current_block_id = then_bb;
                    let then_term = self.lower_block_stmts(then_branch);
                    if !then_term {
                        let then_insts = std::mem::take(&mut self.current_instructions);
                        self.blocks.push(BasicBlock {
                            id: self.current_block_id,
                            params: Vec::new(),
                            instructions: then_insts,
                            terminator: Terminator::Branch(merge_bb, Vec::new(), *span),
                        });
                    }

                    // Else block
                    let mut else_term = false;
                    if let Some(eb) = else_branch {
                        self.current_block_id = else_bb;
                        else_term = self.lower_block_stmts(eb);
                        if !else_term {
                            let else_insts = std::mem::take(&mut self.current_instructions);
                            self.blocks.push(BasicBlock {
                                id: self.current_block_id,
                                params: Vec::new(),
                                instructions: else_insts,
                                terminator: Terminator::Branch(merge_bb, Vec::new(), *span),
                            });
                        }
                    }

                    if then_term && else_term {
                        // Both branches returned, execution won't reach merge_bb
                        return true;
                    }

                    self.current_block_id = merge_bb;
                }
                Stmt::While {
                    condition,
                    body,
                    span,
                } => {
                    let header_bb = self.alloc_block();
                    let body_bb = self.alloc_block();
                    let exit_bb = self.alloc_block();

                    let insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: self.current_block_id,
                        params: Vec::new(),
                        instructions: insts,
                        terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                    });

                    self.current_block_id = header_bb;
                    let cond_val = self.lower_expression(condition);
                    let header_insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: header_bb,
                        params: Vec::new(),
                        instructions: header_insts,
                        terminator: Terminator::CondBranch {
                            cond: cond_val,
                            then_block: body_bb,
                            then_args: Vec::new(),
                            else_block: exit_bb,
                            else_args: Vec::new(),
                            span: *span,
                        },
                    });

                    self.current_block_id = body_bb;
                    let body_term = self.lower_block_stmts(body);
                    if !body_term {
                        let body_insts = std::mem::take(&mut self.current_instructions);
                        self.blocks.push(BasicBlock {
                            id: self.current_block_id,
                            params: Vec::new(),
                            instructions: body_insts,
                            terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                        });
                    }

                    self.current_block_id = exit_bb;
                }
                Stmt::For {
                    init,
                    condition,
                    update,
                    body,
                    span,
                } => {
                    if let Some(i) = init {
                        self.lower_block_stmts(&Block {
                            statements: vec![*i.clone()],
                            span: *span,
                        });
                    }

                    let header_bb = self.alloc_block();
                    let body_bb = self.alloc_block();
                    let exit_bb = self.alloc_block();

                    let insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: self.current_block_id,
                        params: Vec::new(),
                        instructions: insts,
                        terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                    });

                    self.current_block_id = header_bb;
                    let cond_val = if let Some(c) = condition {
                        self.lower_expression(c)
                    } else {
                        let b_val = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: b_val,
                            ty: Type::Bool,
                            op: Op::ConstBool(true),
                            span: *span,
                        });
                        self.value_types.insert(b_val, Type::Bool);
                        b_val
                    };

                    let header_insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: header_bb,
                        params: Vec::new(),
                        instructions: header_insts,
                        terminator: Terminator::CondBranch {
                            cond: cond_val,
                            then_block: body_bb,
                            then_args: Vec::new(),
                            else_block: exit_bb,
                            else_args: Vec::new(),
                            span: *span,
                        },
                    });

                    self.current_block_id = body_bb;
                    let body_term = self.lower_block_stmts(body);
                    if !body_term {
                        if let Some(u) = update {
                            self.lower_block_stmts(&Block {
                                statements: vec![*u.clone()],
                                span: *span,
                            });
                        }
                        let body_insts = std::mem::take(&mut self.current_instructions);
                        self.blocks.push(BasicBlock {
                            id: self.current_block_id,
                            params: Vec::new(),
                            instructions: body_insts,
                            terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                        });
                    }

                    self.current_block_id = exit_bb;
                }
                Stmt::ForIn {
                    item_name,
                    iterable,
                    body,
                    span,
                } => {
                    let iter_val = self.lower_expression(iterable);
                    let elem_ty = match self.value_types.get(&iter_val) {
                        Some(Type::Array(inner)) => (**inner).clone(),
                        _ => {
                            if let Expr::Variable(var_name, _) = iterable {
                                match self.variable_types.get(var_name) {
                                    Some(Type::Array(inner)) => (**inner).clone(),
                                    _ => Type::Int,
                                }
                            } else {
                                Type::Int
                            }
                        }
                    };

                    let len_val = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: len_val,
                        ty: Type::Int,
                        op: Op::ArrayLength { array: iter_val },
                        span: *span,
                    });
                    self.value_types.insert(len_val, Type::Int);

                    let idx_slot = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: idx_slot,
                        ty: Type::Int,
                        op: Op::Alloc {
                            domain: MemoryDomain::Stack,
                            ty: Type::Int,
                        },
                        span: *span,
                    });
                    self.value_types.insert(idx_slot, Type::Int);

                    let zero_val = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: zero_val,
                        ty: Type::Int,
                        op: Op::ConstInt(0),
                        span: *span,
                    });
                    self.value_types.insert(zero_val, Type::Int);

                    let store_zero = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: store_zero,
                        ty: Type::Void,
                        op: Op::Store {
                            ptr: idx_slot,
                            val: zero_val,
                        },
                        span: *span,
                    });
                    self.value_types.insert(store_zero, Type::Void);

                    let header_bb = self.alloc_block();
                    let body_bb = self.alloc_block();
                    let exit_bb = self.alloc_block();

                    let insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: self.current_block_id,
                        params: Vec::new(),
                        instructions: insts,
                        terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                    });

                    self.current_block_id = header_bb;
                    let curr_idx = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: curr_idx,
                        ty: Type::Int,
                        op: Op::Load { ptr: idx_slot },
                        span: *span,
                    });
                    self.value_types.insert(curr_idx, Type::Int);

                    let cond_val = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: cond_val,
                        ty: Type::Bool,
                        op: Op::Binary {
                            op: BinaryOp::Less,
                            left: curr_idx,
                            right: len_val,
                        },
                        span: *span,
                    });
                    self.value_types.insert(cond_val, Type::Bool);

                    let header_insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: header_bb,
                        params: Vec::new(),
                        instructions: header_insts,
                        terminator: Terminator::CondBranch {
                            cond: cond_val,
                            then_block: body_bb,
                            then_args: Vec::new(),
                            else_block: exit_bb,
                            else_args: Vec::new(),
                            span: *span,
                        },
                    });

                    self.current_block_id = body_bb;
                    let elem_val = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: elem_val,
                        ty: elem_ty.clone(),
                        op: Op::GetIndex {
                            array: iter_val,
                            index: curr_idx,
                        },
                        span: *span,
                    });
                    self.value_types.insert(elem_val, elem_ty.clone());

                    let item_slot = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: item_slot,
                        ty: elem_ty.clone(),
                        op: Op::Alloc {
                            domain: MemoryDomain::Stack,
                            ty: elem_ty.clone(),
                        },
                        span: *span,
                    });
                    self.value_types.insert(item_slot, elem_ty.clone());

                    let store_item = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: store_item,
                        ty: Type::Void,
                        op: Op::Store {
                            ptr: item_slot,
                            val: elem_val,
                        },
                        span: *span,
                    });
                    self.value_types.insert(store_item, Type::Void);

                    self.variables.insert(item_name.clone(), item_slot);
                    self.variable_types.insert(item_name.clone(), elem_ty);

                    let body_term = self.lower_block_stmts(body);
                    if !body_term {
                        let one_val = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: one_val,
                            ty: Type::Int,
                            op: Op::ConstInt(1),
                            span: *span,
                        });
                        self.value_types.insert(one_val, Type::Int);

                        let loop_idx = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: loop_idx,
                            ty: Type::Int,
                            op: Op::Load { ptr: idx_slot },
                            span: *span,
                        });
                        self.value_types.insert(loop_idx, Type::Int);

                        let next_idx = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: next_idx,
                            ty: Type::Int,
                            op: Op::Binary {
                                op: BinaryOp::Add,
                                left: loop_idx,
                                right: one_val,
                            },
                            span: *span,
                        });
                        self.value_types.insert(next_idx, Type::Int);

                        let store_next = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: store_next,
                            ty: Type::Void,
                            op: Op::Store {
                                ptr: idx_slot,
                                val: next_idx,
                            },
                            span: *span,
                        });
                        self.value_types.insert(store_next, Type::Void);

                        let body_insts = std::mem::take(&mut self.current_instructions);
                        self.blocks.push(BasicBlock {
                            id: self.current_block_id,
                            params: Vec::new(),
                            instructions: body_insts,
                            terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                        });
                    }

                    self.current_block_id = exit_bb;
                }
                Stmt::Throw { expr, span } => {
                    let val = self.lower_expression(expr);
                    let val_ty = self.value_types.get(&val).cloned().unwrap_or(Type::String);
                    let str_val = if val_ty == Type::String {
                        val
                    } else if val_ty == Type::Int {
                        let res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: Type::String,
                            op: Op::Call {
                                func_name: "syntra_int_to_str".to_string(),
                                args: vec![val],
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, Type::String);
                        res
                    } else if val_ty == Type::Bool {
                        let res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: Type::String,
                            op: Op::Call {
                                func_name: "syntra_bool_to_str".to_string(),
                                args: vec![val],
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, Type::String);
                        res
                    } else {
                        val
                    };

                    let err_res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: err_res,
                        ty: Type::Void,
                        op: Op::Call {
                            func_name: "syntra_set_error".to_string(),
                            args: vec![str_val],
                        },
                        span: *span,
                    });
                    self.value_types.insert(err_res, Type::Void);

                    if let Some(ctx) = self.try_stack.last() {
                        let target = ctx.catch_block;
                        let insts = std::mem::take(&mut self.current_instructions);
                        self.blocks.push(BasicBlock {
                            id: self.current_block_id,
                            params: Vec::new(),
                            instructions: insts,
                            terminator: Terminator::Branch(target, Vec::new(), *span),
                        });
                        self.current_block_id = self.alloc_block();
                        return true;
                    } else {
                        let print_res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: print_res,
                            ty: Type::Void,
                            op: Op::Call {
                                func_name: "syntra_println_str".to_string(),
                                args: vec![str_val],
                            },
                            span: *span,
                        });
                        let code = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: code,
                            ty: Type::Int,
                            op: Op::ConstInt(1),
                            span: *span,
                        });
                        let exit_res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: exit_res,
                            ty: Type::Void,
                            op: Op::Call {
                                func_name: "syntra_system_exit".to_string(),
                                args: vec![code],
                            },
                            span: *span,
                        });
                        let insts = std::mem::take(&mut self.current_instructions);
                        self.blocks.push(BasicBlock {
                            id: self.current_block_id,
                            params: Vec::new(),
                            instructions: insts,
                            terminator: Terminator::Return(None, *span),
                        });
                        self.current_block_id = self.alloc_block();
                        return true;
                    }
                }
                Stmt::TryCatch {
                    try_block,
                    catch_param,
                    catch_block,
                    finally_block,
                    span,
                } => {
                    let try_bb = self.alloc_block();
                    let catch_bb = self.alloc_block();
                    let finally_bb = self.alloc_block();
                    let after_bb = self.alloc_block();

                    let catch_target = if catch_block.is_some() {
                        catch_bb
                    } else if finally_block.is_some() {
                        finally_bb
                    } else {
                        after_bb
                    };
                    let next_normal = if finally_block.is_some() {
                        finally_bb
                    } else {
                        after_bb
                    };

                    let insts = std::mem::take(&mut self.current_instructions);
                    self.blocks.push(BasicBlock {
                        id: self.current_block_id,
                        params: Vec::new(),
                        instructions: insts,
                        terminator: Terminator::Branch(try_bb, Vec::new(), *span),
                    });

                    self.current_block_id = try_bb;
                    self.try_stack.push(TryContext {
                        catch_block: catch_target,
                        finally_block: if finally_block.is_some() {
                            Some(finally_bb)
                        } else {
                            None
                        },
                    });
                    let try_terminated = self.lower_block_stmts(try_block);
                    self.try_stack.pop();

                    if !try_terminated {
                        let insts = std::mem::take(&mut self.current_instructions);
                        self.blocks.push(BasicBlock {
                            id: self.current_block_id,
                            params: Vec::new(),
                            instructions: insts,
                            terminator: Terminator::Branch(next_normal, Vec::new(), *span),
                        });
                    }

                    if let Some(cb) = catch_block {
                        self.current_block_id = catch_bb;
                        if let Some(cp) = catch_param {
                            let err_val = self.alloc_val();
                            self.current_instructions.push(Instruction {
                                result: err_val,
                                ty: Type::String,
                                op: Op::Call {
                                    func_name: "syntra_get_error".to_string(),
                                    args: Vec::new(),
                                },
                                span: *span,
                            });
                            self.value_types.insert(err_val, Type::String);

                            let slot = self.alloc_val();
                            self.current_instructions.push(Instruction {
                                result: slot,
                                ty: Type::String,
                                op: Op::Alloc {
                                    domain: MemoryDomain::Stack,
                                    ty: Type::String,
                                },
                                span: *span,
                            });
                            self.value_types.insert(slot, Type::String);

                            let store_res = self.alloc_val();
                            self.current_instructions.push(Instruction {
                                result: store_res,
                                ty: Type::Void,
                                op: Op::Store {
                                    ptr: slot,
                                    val: err_val,
                                },
                                span: *span,
                            });
                            self.value_types.insert(store_res, Type::Void);

                            self.variables.insert(cp.clone(), slot);
                            self.variable_types.insert(cp.clone(), Type::String);
                        }

                        let clear_res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: clear_res,
                            ty: Type::Void,
                            op: Op::Call {
                                func_name: "syntra_clear_error".to_string(),
                                args: Vec::new(),
                            },
                            span: *span,
                        });
                        self.value_types.insert(clear_res, Type::Void);

                        let catch_terminated = self.lower_block_stmts(cb);
                        if !catch_terminated {
                            let insts = std::mem::take(&mut self.current_instructions);
                            self.blocks.push(BasicBlock {
                                id: self.current_block_id,
                                params: Vec::new(),
                                instructions: insts,
                                terminator: Terminator::Branch(next_normal, Vec::new(), *span),
                            });
                        }
                    }

                    if let Some(fb) = finally_block {
                        self.current_block_id = finally_bb;
                        let fin_terminated = self.lower_block_stmts(fb);
                        if !fin_terminated {
                            let insts = std::mem::take(&mut self.current_instructions);
                            self.blocks.push(BasicBlock {
                                id: self.current_block_id,
                                params: Vec::new(),
                                instructions: insts,
                                terminator: Terminator::Branch(after_bb, Vec::new(), *span),
                            });
                        }
                    }

                    self.current_block_id = after_bb;
                }
            }
        }
        false
    }

    fn get_object_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::This(_) => self.current_class.as_ref().map(|c| Type::Class(c.clone())),
            Expr::Variable(name, _) => self
                .variable_types
                .get(name)
                .cloned()
                .or_else(|| self.parent_variable_types.get(name).cloned()),
            Expr::New {
                class_name,
                generic_args,
                ..
            } => {
                if !generic_args.is_empty() {
                    Some(Type::GenericInstance(
                        class_name.clone(),
                        generic_args.iter().map(Type::from_ast).collect(),
                    ))
                } else {
                    Some(Type::Class(class_name.clone()))
                }
            }
            Expr::Index { array, .. } => {
                if let Some(Type::Array(inner)) = self.get_object_type(array) {
                    return Some((*inner).clone());
                }
                if let Expr::Variable(name, _) = array.as_ref() {
                    if let Some(Type::Array(inner)) = self
                        .variable_types
                        .get(name)
                        .or_else(|| self.parent_variable_types.get(name))
                    {
                        return Some((**inner).clone());
                    }
                }
                None
            }
            Expr::MemberAccess { object, member, .. } => {
                if let Some(parent_cls) = self.get_object_class(object) {
                    if let Some(layout) = self.class_layouts.get(&parent_cls) {
                        if let Some(field) = layout.fields.get(member) {
                            return Some(field.ty.clone());
                        }
                    }
                }
                None
            }
            Expr::Call { callee, .. } => {
                if let Expr::MemberAccess { object, member, .. } = callee.as_ref() {
                    if let Some(parent_cls) = self.get_object_class(object) {
                        if let Some(ret_ty) = self
                            .method_return_types
                            .get(&(parent_cls.clone(), member.clone()))
                        {
                            return Some(ret_ty.clone());
                        }
                    }
                } else if let Expr::Variable(fn_name, _) = callee.as_ref() {
                    if let Some(ret_ty) = self.function_return_types.get(fn_name) {
                        return Some(ret_ty.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn get_object_class(&self, expr: &Expr) -> Option<String> {
        match self.get_object_type(expr) {
            Some(Type::Class(cls)) => Some(cls),
            Some(Type::GenericInstance(cls, _)) => Some(cls),
            _ => None,
        }
    }

    fn lower_expression(&mut self, expr: &Expr) -> ValueId {
        match expr {
            Expr::Integer(i, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Int,
                    op: Op::ConstInt(*i),
                    span: *span,
                });
                self.value_types.insert(res, Type::Int);
                res
            }
            Expr::Float(f, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Float,
                    op: Op::ConstFloat(*f),
                    span: *span,
                });
                self.value_types.insert(res, Type::Float);
                res
            }
            Expr::BoolLit(b, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Bool,
                    op: Op::ConstBool(*b),
                    span: *span,
                });
                self.value_types.insert(res, Type::Bool);
                res
            }
            Expr::StringLit(s, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::String,
                    op: Op::ConstString(s.clone()),
                    span: *span,
                });
                self.value_types.insert(res, Type::String);
                res
            }
            Expr::Variable(name, span) => {
                if let Some(id) = self.variables.get(name).copied() {
                    if name == "this" {
                        id
                    } else {
                        let var_ty = self.variable_types.get(name).cloned().unwrap_or(Type::Int);
                        let res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: var_ty.clone(),
                            op: Op::Load { ptr: id },
                            span: *span,
                        });
                        self.value_types.insert(res, var_ty);
                        res
                    }
                } else if self.function_return_types.contains_key(name) {
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: Type::Int,
                        op: Op::FuncAddr {
                            func_name: name.clone(),
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, Type::Int);
                    res
                } else if self.class_layouts.contains_key(name) {
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: Type::String,
                        op: Op::ConstString(name.clone()),
                        span: *span,
                    });
                    self.value_types.insert(res, Type::String);
                    res
                } else if let Some(var_ty) = self.parent_variable_types.get(name).cloned() {
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: var_ty.clone(),
                        op: Op::ConstInt(0),
                        span: *span,
                    });
                    self.value_types.insert(res, var_ty);
                    res
                } else {
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: Type::Int,
                        op: Op::ConstInt(0),
                        span: *span,
                    });
                    self.value_types.insert(res, Type::Int);
                    res
                }
            }
            Expr::This(span) => {
                if let Some(id) = self.variables.get("this") {
                    *id
                } else {
                    let res = self.alloc_val();
                    let cls_ty = self
                        .current_class
                        .as_ref()
                        .map(|c| Type::Class(c.clone()))
                        .unwrap_or(Type::Void);
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: cls_ty.clone(),
                        op: Op::ConstInt(0),
                        span: *span,
                    });
                    self.value_types.insert(res, cls_ty);
                    res
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let l_val = self.lower_expression(left);
                let r_val = self.lower_expression(right);

                let l_ty = self.value_types.get(&l_val).cloned().unwrap_or(Type::Int);
                let r_ty = self.value_types.get(&r_val).cloned().unwrap_or(Type::Int);

                // Handle string concatenation
                if *op == BinaryOp::Add && (l_ty == Type::String || r_ty == Type::String) {
                    let actual_l = if l_ty == Type::String {
                        l_val
                    } else {
                        let fn_name = if l_ty == Type::Bool {
                            "syntra_bool_to_str"
                        } else {
                            "syntra_int_to_str"
                        };
                        let conv_val = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: conv_val,
                            ty: Type::String,
                            op: Op::Call {
                                func_name: fn_name.to_string(),
                                args: vec![l_val],
                            },
                            span: *span,
                        });
                        self.value_types.insert(conv_val, Type::String);
                        conv_val
                    };

                    let actual_r = if r_ty == Type::String {
                        r_val
                    } else {
                        let fn_name = if r_ty == Type::Bool {
                            "syntra_bool_to_str"
                        } else {
                            "syntra_int_to_str"
                        };
                        let conv_val = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: conv_val,
                            ty: Type::String,
                            op: Op::Call {
                                func_name: fn_name.to_string(),
                                args: vec![r_val],
                            },
                            span: *span,
                        });
                        self.value_types.insert(conv_val, Type::String);
                        conv_val
                    };

                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: Type::String,
                        op: Op::Call {
                            func_name: "syntra_concat_str".to_string(),
                            args: vec![actual_l, actual_r],
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, Type::String);
                    return res;
                }

                // Handle string equality comparison
                if *op == BinaryOp::EqualEqual && (l_ty == Type::String || r_ty == Type::String) {
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: Type::Bool,
                        op: Op::Call {
                            func_name: "syntra_str_eq".to_string(),
                            args: vec![l_val, r_val],
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, Type::Bool);
                    return res;
                }
                if *op == BinaryOp::NotEqual && (l_ty == Type::String || r_ty == Type::String) {
                    let eq_res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: eq_res,
                        ty: Type::Bool,
                        op: Op::Call {
                            func_name: "syntra_str_eq".to_string(),
                            args: vec![l_val, r_val],
                        },
                        span: *span,
                    });
                    self.value_types.insert(eq_res, Type::Bool);
                    let zero = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: zero,
                        ty: Type::Bool,
                        op: Op::ConstBool(false),
                        span: *span,
                    });
                    self.value_types.insert(zero, Type::Bool);
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: Type::Bool,
                        op: Op::Binary {
                            op: BinaryOp::EqualEqual,
                            left: eq_res,
                            right: zero,
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, Type::Bool);
                    return res;
                }

                let res = self.alloc_val();
                let ty = match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => Type::Int,
                    _ => Type::Bool,
                };

                self.current_instructions.push(Instruction {
                    result: res,
                    ty: ty.clone(),
                    op: Op::Binary {
                        op: *op,
                        left: l_val,
                        right: r_val,
                    },
                    span: *span,
                });
                self.value_types.insert(res, ty);
                res
            }
            Expr::Call { callee, args, span } => {
                if let Expr::MemberAccess { object, member, .. } = callee.as_ref() {
                    if (member == "map"
                        || member == "filter"
                        || member == "reduce"
                        || member == "forEach")
                        && !args.is_empty()
                    {
                        if let Expr::Lambda {
                            params,
                            body_expr,
                            body_block,
                            span: _,
                        } = &args[0]
                        {
                            let arr_val = self.lower_expression(object);
                            let arr_len = self.alloc_val();
                            self.current_instructions.push(Instruction {
                                result: arr_len,
                                ty: Type::Int,
                                op: Op::ArrayLength { array: arr_val },
                                span: *span,
                            });
                            self.value_types.insert(arr_len, Type::Int);

                            let p_name = params
                                .first()
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| "x".to_string());

                            if member == "map" {
                                let new_arr = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: new_arr,
                                    ty: Type::Array(Box::new(Type::Int)),
                                    op: Op::AllocArray {
                                        elem_ty: Type::Int,
                                        len: arr_len,
                                    },
                                    span: *span,
                                });
                                self.value_types
                                    .insert(new_arr, Type::Array(Box::new(Type::Int)));

                                let idx_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: idx_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(idx_slot, Type::Int);

                                let zero = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: zero,
                                    ty: Type::Int,
                                    op: Op::ConstInt(0),
                                    span: *span,
                                });
                                self.value_types.insert(zero, Type::Int);

                                let st = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: zero,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st, Type::Void);

                                let header_bb = self.alloc_block();
                                let body_bb = self.alloc_block();
                                let exit_bb = self.alloc_block();

                                let insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: self.current_block_id,
                                    params: Vec::new(),
                                    instructions: insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = header_bb;
                                let curr_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: curr_idx,
                                    ty: Type::Int,
                                    op: Op::Load { ptr: idx_slot },
                                    span: *span,
                                });
                                self.value_types.insert(curr_idx, Type::Int);

                                let cond = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: cond,
                                    ty: Type::Bool,
                                    op: Op::Binary {
                                        op: BinaryOp::Less,
                                        left: curr_idx,
                                        right: arr_len,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(cond, Type::Bool);

                                let h_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: header_bb,
                                    params: Vec::new(),
                                    instructions: h_insts,
                                    terminator: Terminator::CondBranch {
                                        cond,
                                        then_block: body_bb,
                                        then_args: Vec::new(),
                                        else_block: exit_bb,
                                        else_args: Vec::new(),
                                        span: *span,
                                    },
                                });

                                self.current_block_id = body_bb;
                                let elem_val = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: elem_val,
                                    ty: Type::Int,
                                    op: Op::GetIndex {
                                        array: arr_val,
                                        index: curr_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(elem_val, Type::Int);

                                let p_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_slot, Type::Int);

                                let p_st = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_st,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: p_slot,
                                        val: elem_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_st, Type::Void);

                                let old_var = self.variables.insert(p_name.clone(), p_slot);
                                let old_ty = self.variable_types.insert(p_name.clone(), Type::Int);

                                let mapped_val = if let Some(be) = body_expr {
                                    self.lower_expression(be)
                                } else if let Some(bb) = body_block {
                                    self.lower_block_stmts(bb);
                                    let dummy = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy,
                                        ty: Type::Int,
                                        op: Op::ConstInt(0),
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy, Type::Int);
                                    dummy
                                } else {
                                    elem_val
                                };

                                if let Some(ov) = old_var {
                                    self.variables.insert(p_name.clone(), ov);
                                } else {
                                    self.variables.remove(&p_name);
                                }
                                if let Some(ot) = old_ty {
                                    self.variable_types.insert(p_name.clone(), ot);
                                } else {
                                    self.variable_types.remove(&p_name);
                                }

                                let set_res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: set_res,
                                    ty: Type::Void,
                                    op: Op::SetIndex {
                                        array: new_arr,
                                        index: curr_idx,
                                        val: mapped_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(set_res, Type::Void);

                                let one = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: one,
                                    ty: Type::Int,
                                    op: Op::ConstInt(1),
                                    span: *span,
                                });
                                self.value_types.insert(one, Type::Int);

                                let next_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: next_idx,
                                    ty: Type::Int,
                                    op: Op::Binary {
                                        op: BinaryOp::Add,
                                        left: curr_idx,
                                        right: one,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(next_idx, Type::Int);

                                let st_next = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st_next,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: next_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st_next, Type::Void);

                                let b_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: body_bb,
                                    params: Vec::new(),
                                    instructions: b_insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = exit_bb;
                                return new_arr;
                            } else if member == "filter" {
                                let new_arr = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: new_arr,
                                    ty: Type::Array(Box::new(Type::Int)),
                                    op: Op::AllocArray {
                                        elem_ty: Type::Int,
                                        len: arr_len,
                                    },
                                    span: *span,
                                });
                                self.value_types
                                    .insert(new_arr, Type::Array(Box::new(Type::Int)));

                                let idx_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: idx_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(idx_slot, Type::Int);

                                let write_idx_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: write_idx_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(write_idx_slot, Type::Int);

                                let zero = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: zero,
                                    ty: Type::Int,
                                    op: Op::ConstInt(0),
                                    span: *span,
                                });
                                self.value_types.insert(zero, Type::Int);

                                let st1 = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st1,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: zero,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st1, Type::Void);

                                let st2 = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st2,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: write_idx_slot,
                                        val: zero,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st2, Type::Void);

                                let header_bb = self.alloc_block();
                                let body_bb = self.alloc_block();
                                let exit_bb = self.alloc_block();

                                let insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: self.current_block_id,
                                    params: Vec::new(),
                                    instructions: insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = header_bb;
                                let curr_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: curr_idx,
                                    ty: Type::Int,
                                    op: Op::Load { ptr: idx_slot },
                                    span: *span,
                                });
                                self.value_types.insert(curr_idx, Type::Int);

                                let cond = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: cond,
                                    ty: Type::Bool,
                                    op: Op::Binary {
                                        op: BinaryOp::Less,
                                        left: curr_idx,
                                        right: arr_len,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(cond, Type::Bool);

                                let h_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: header_bb,
                                    params: Vec::new(),
                                    instructions: h_insts,
                                    terminator: Terminator::CondBranch {
                                        cond,
                                        then_block: body_bb,
                                        then_args: Vec::new(),
                                        else_block: exit_bb,
                                        else_args: Vec::new(),
                                        span: *span,
                                    },
                                });

                                self.current_block_id = body_bb;
                                let elem_val = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: elem_val,
                                    ty: Type::Int,
                                    op: Op::GetIndex {
                                        array: arr_val,
                                        index: curr_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(elem_val, Type::Int);

                                let p_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_slot, Type::Int);

                                let p_st = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_st,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: p_slot,
                                        val: elem_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_st, Type::Void);

                                let old_var = self.variables.insert(p_name.clone(), p_slot);
                                let old_ty = self.variable_types.insert(p_name.clone(), Type::Int);

                                let pred_val = if let Some(be) = body_expr {
                                    self.lower_expression(be)
                                } else {
                                    elem_val
                                };

                                if let Some(ov) = old_var {
                                    self.variables.insert(p_name.clone(), ov);
                                } else {
                                    self.variables.remove(&p_name);
                                }
                                if let Some(ot) = old_ty {
                                    self.variable_types.insert(p_name.clone(), ot);
                                } else {
                                    self.variable_types.remove(&p_name);
                                }

                                let write_bb = self.alloc_block();
                                let adv_bb = self.alloc_block();

                                let b_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: body_bb,
                                    params: Vec::new(),
                                    instructions: b_insts,
                                    terminator: Terminator::CondBranch {
                                        cond: pred_val,
                                        then_block: write_bb,
                                        then_args: Vec::new(),
                                        else_block: adv_bb,
                                        else_args: Vec::new(),
                                        span: *span,
                                    },
                                });

                                // write_bb: write element and advance write index
                                self.current_block_id = write_bb;
                                let wr_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: wr_idx,
                                    ty: Type::Int,
                                    op: Op::Load {
                                        ptr: write_idx_slot,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(wr_idx, Type::Int);

                                let set_res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: set_res,
                                    ty: Type::Void,
                                    op: Op::SetIndex {
                                        array: new_arr,
                                        index: wr_idx,
                                        val: elem_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(set_res, Type::Void);

                                let one_wr = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: one_wr,
                                    ty: Type::Int,
                                    op: Op::ConstInt(1),
                                    span: *span,
                                });
                                self.value_types.insert(one_wr, Type::Int);

                                let next_wr_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: next_wr_idx,
                                    ty: Type::Int,
                                    op: Op::Binary {
                                        op: BinaryOp::Add,
                                        left: wr_idx,
                                        right: one_wr,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(next_wr_idx, Type::Int);

                                let st_wr = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st_wr,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: write_idx_slot,
                                        val: next_wr_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st_wr, Type::Void);

                                let w_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: write_bb,
                                    params: Vec::new(),
                                    instructions: w_insts,
                                    terminator: Terminator::Branch(adv_bb, Vec::new(), *span),
                                });

                                // adv_bb: advance array index and branch back to header
                                self.current_block_id = adv_bb;
                                let one_adv = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: one_adv,
                                    ty: Type::Int,
                                    op: Op::ConstInt(1),
                                    span: *span,
                                });
                                self.value_types.insert(one_adv, Type::Int);

                                let next_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: next_idx,
                                    ty: Type::Int,
                                    op: Op::Binary {
                                        op: BinaryOp::Add,
                                        left: curr_idx,
                                        right: one_adv,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(next_idx, Type::Int);

                                let st_next = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st_next,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: next_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st_next, Type::Void);

                                let adv_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: adv_bb,
                                    params: Vec::new(),
                                    instructions: adv_insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = exit_bb;
                                let final_len = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: final_len,
                                    ty: Type::Int,
                                    op: Op::Load {
                                        ptr: write_idx_slot,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(final_len, Type::Int);

                                let set_len_res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: set_len_res,
                                    ty: Type::Void,
                                    op: Op::Call {
                                        func_name: "syntra_array_set_len".to_string(),
                                        args: vec![new_arr, final_len],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(set_len_res, Type::Void);

                                return new_arr;
                            } else if member == "reduce" {
                                let init_val = if args.len() >= 2 {
                                    self.lower_expression(&args[1])
                                } else {
                                    let zero = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: zero,
                                        ty: Type::Int,
                                        op: Op::ConstInt(0),
                                        span: *span,
                                    });
                                    self.value_types.insert(zero, Type::Int);
                                    zero
                                };

                                let acc_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: acc_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(acc_slot, Type::Int);

                                let st_acc = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st_acc,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: acc_slot,
                                        val: init_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st_acc, Type::Void);

                                let idx_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: idx_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(idx_slot, Type::Int);

                                let zero = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: zero,
                                    ty: Type::Int,
                                    op: Op::ConstInt(0),
                                    span: *span,
                                });
                                self.value_types.insert(zero, Type::Int);

                                let st = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: zero,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st, Type::Void);

                                let header_bb = self.alloc_block();
                                let body_bb = self.alloc_block();
                                let exit_bb = self.alloc_block();

                                let insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: self.current_block_id,
                                    params: Vec::new(),
                                    instructions: insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = header_bb;
                                let curr_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: curr_idx,
                                    ty: Type::Int,
                                    op: Op::Load { ptr: idx_slot },
                                    span: *span,
                                });
                                self.value_types.insert(curr_idx, Type::Int);

                                let cond = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: cond,
                                    ty: Type::Bool,
                                    op: Op::Binary {
                                        op: BinaryOp::Less,
                                        left: curr_idx,
                                        right: arr_len,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(cond, Type::Bool);

                                let h_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: header_bb,
                                    params: Vec::new(),
                                    instructions: h_insts,
                                    terminator: Terminator::CondBranch {
                                        cond,
                                        then_block: body_bb,
                                        then_args: Vec::new(),
                                        else_block: exit_bb,
                                        else_args: Vec::new(),
                                        span: *span,
                                    },
                                });

                                self.current_block_id = body_bb;
                                let elem_val = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: elem_val,
                                    ty: Type::Int,
                                    op: Op::GetIndex {
                                        array: arr_val,
                                        index: curr_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(elem_val, Type::Int);

                                let acc_val = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: acc_val,
                                    ty: Type::Int,
                                    op: Op::Load { ptr: acc_slot },
                                    span: *span,
                                });
                                self.value_types.insert(acc_val, Type::Int);

                                let acc_p_name = params
                                    .first()
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| "acc".to_string());
                                let elem_p_name = params
                                    .get(1)
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| "elem".to_string());

                                let acc_p_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: acc_p_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(acc_p_slot, Type::Int);

                                let p_st1 = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_st1,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: acc_p_slot,
                                        val: acc_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_st1, Type::Void);

                                let elem_p_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: elem_p_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(elem_p_slot, Type::Int);

                                let p_st2 = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_st2,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: elem_p_slot,
                                        val: elem_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_st2, Type::Void);

                                let old_acc_var =
                                    self.variables.insert(acc_p_name.clone(), acc_p_slot);
                                let old_elem_var =
                                    self.variables.insert(elem_p_name.clone(), elem_p_slot);

                                let new_acc = if let Some(be) = body_expr {
                                    self.lower_expression(be)
                                } else {
                                    acc_val
                                };

                                if let Some(ov) = old_acc_var {
                                    self.variables.insert(acc_p_name.clone(), ov);
                                } else {
                                    self.variables.remove(&acc_p_name);
                                }
                                if let Some(ov) = old_elem_var {
                                    self.variables.insert(elem_p_name.clone(), ov);
                                } else {
                                    self.variables.remove(&elem_p_name);
                                }

                                let st_new_acc = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st_new_acc,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: acc_slot,
                                        val: new_acc,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st_new_acc, Type::Void);

                                let one = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: one,
                                    ty: Type::Int,
                                    op: Op::ConstInt(1),
                                    span: *span,
                                });
                                self.value_types.insert(one, Type::Int);

                                let next_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: next_idx,
                                    ty: Type::Int,
                                    op: Op::Binary {
                                        op: BinaryOp::Add,
                                        left: curr_idx,
                                        right: one,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(next_idx, Type::Int);

                                let st_next = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st_next,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: next_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st_next, Type::Void);

                                let b_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: body_bb,
                                    params: Vec::new(),
                                    instructions: b_insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = exit_bb;
                                let final_acc = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: final_acc,
                                    ty: Type::Int,
                                    op: Op::Load { ptr: acc_slot },
                                    span: *span,
                                });
                                self.value_types.insert(final_acc, Type::Int);
                                return final_acc;
                            } else if member == "forEach" {
                                let idx_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: idx_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(idx_slot, Type::Int);

                                let zero = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: zero,
                                    ty: Type::Int,
                                    op: Op::ConstInt(0),
                                    span: *span,
                                });
                                self.value_types.insert(zero, Type::Int);

                                let st = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: zero,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st, Type::Void);

                                let header_bb = self.alloc_block();
                                let body_bb = self.alloc_block();
                                let exit_bb = self.alloc_block();

                                let insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: self.current_block_id,
                                    params: Vec::new(),
                                    instructions: insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = header_bb;
                                let curr_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: curr_idx,
                                    ty: Type::Int,
                                    op: Op::Load { ptr: idx_slot },
                                    span: *span,
                                });
                                self.value_types.insert(curr_idx, Type::Int);

                                let cond = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: cond,
                                    ty: Type::Bool,
                                    op: Op::Binary {
                                        op: BinaryOp::Less,
                                        left: curr_idx,
                                        right: arr_len,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(cond, Type::Bool);

                                let h_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: header_bb,
                                    params: Vec::new(),
                                    instructions: h_insts,
                                    terminator: Terminator::CondBranch {
                                        cond,
                                        then_block: body_bb,
                                        then_args: Vec::new(),
                                        else_block: exit_bb,
                                        else_args: Vec::new(),
                                        span: *span,
                                    },
                                });

                                self.current_block_id = body_bb;
                                let elem_val = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: elem_val,
                                    ty: Type::Int,
                                    op: Op::GetIndex {
                                        array: arr_val,
                                        index: curr_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(elem_val, Type::Int);

                                let p_slot = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_slot,
                                    ty: Type::Int,
                                    op: Op::Alloc {
                                        domain: MemoryDomain::Stack,
                                        ty: Type::Int,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_slot, Type::Int);

                                let p_st = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: p_st,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: p_slot,
                                        val: elem_val,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(p_st, Type::Void);

                                let old_var = self.variables.insert(p_name.clone(), p_slot);
                                let old_ty = self.variable_types.insert(p_name.clone(), Type::Int);

                                if let Some(be) = body_expr {
                                    self.lower_expression(be);
                                } else if let Some(bb) = body_block {
                                    self.lower_block_stmts(bb);
                                }

                                if let Some(ov) = old_var {
                                    self.variables.insert(p_name.clone(), ov);
                                } else {
                                    self.variables.remove(&p_name);
                                }
                                if let Some(ot) = old_ty {
                                    self.variable_types.insert(p_name.clone(), ot);
                                } else {
                                    self.variable_types.remove(&p_name);
                                }

                                let one = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: one,
                                    ty: Type::Int,
                                    op: Op::ConstInt(1),
                                    span: *span,
                                });
                                self.value_types.insert(one, Type::Int);

                                let next_idx = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: next_idx,
                                    ty: Type::Int,
                                    op: Op::Binary {
                                        op: BinaryOp::Add,
                                        left: curr_idx,
                                        right: one,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(next_idx, Type::Int);

                                let st_next = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: st_next,
                                    ty: Type::Void,
                                    op: Op::Store {
                                        ptr: idx_slot,
                                        val: next_idx,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(st_next, Type::Void);

                                let b_insts = std::mem::take(&mut self.current_instructions);
                                self.blocks.push(BasicBlock {
                                    id: body_bb,
                                    params: Vec::new(),
                                    instructions: b_insts,
                                    terminator: Terminator::Branch(header_bb, Vec::new(), *span),
                                });

                                self.current_block_id = exit_bb;
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::Void,
                                    op: Op::ConstInt(0),
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::Void);
                                return res;
                            }
                        }
                    }
                }

                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.lower_expression(a));
                }

                match callee.as_ref() {
                    Expr::MemberAccess { object, member, .. } => {
                        // Check for static Math methods
                        if let Expr::Variable(obj_name, _) = object.as_ref() {
                            if obj_name == "Math" {
                                if member == "min" || member == "max" {
                                    let fn_symbol = if member == "min" {
                                        "syntra_math_min"
                                    } else {
                                        "syntra_math_max"
                                    };
                                    let arr_fn_symbol = if member == "min" {
                                        "syntra_array_min"
                                    } else {
                                        "syntra_array_max"
                                    };

                                    if arg_vals.is_empty() {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Int,
                                            op: Op::ConstInt(0),
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Int);
                                        return res;
                                    }

                                    if arg_vals.len() == 1 {
                                        let first_ty = self.value_types.get(&arg_vals[0]);
                                        if matches!(first_ty, Some(Type::Array(_))) {
                                            let res = self.alloc_val();
                                            self.current_instructions.push(Instruction {
                                                result: res,
                                                ty: Type::Int,
                                                op: Op::Call {
                                                    func_name: arr_fn_symbol.to_string(),
                                                    args: vec![arg_vals[0]],
                                                },
                                                span: *span,
                                            });
                                            self.value_types.insert(res, Type::Int);
                                            return res;
                                        }
                                        return arg_vals[0];
                                    }

                                    // Fold multi-arguments with binary calls
                                    let mut curr = arg_vals[0];
                                    for next_val in &arg_vals[1..] {
                                        let fold_res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: fold_res,
                                            ty: Type::Int,
                                            op: Op::Call {
                                                func_name: fn_symbol.to_string(),
                                                args: vec![curr, *next_val],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(fold_res, Type::Int);
                                        curr = fold_res;
                                    }
                                    return curr;
                                }

                                if member == "abs" {
                                    let arg = if let Some(v) = arg_vals.first() {
                                        *v
                                    } else {
                                        let zero = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: zero,
                                            ty: Type::Int,
                                            op: Op::ConstInt(0),
                                            span: *span,
                                        });
                                        self.value_types.insert(zero, Type::Int);
                                        zero
                                    };
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_math_abs".to_string(),
                                            args: vec![arg],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }

                                if member == "clamp" {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_math_clamp".to_string(),
                                            args: arg_vals,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }

                                if member == "pow" {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_math_pow".to_string(),
                                            args: arg_vals,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }

                                if member == "sqrt" {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_math_sqrt".to_string(),
                                            args: arg_vals,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                            }

                            // Check for static File methods
                            if obj_name == "File" {
                                match member.as_str() {
                                    "readText" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::String,
                                            op: Op::Call {
                                                func_name: "syntra_file_read_text".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::String);
                                        return res;
                                    }
                                    "writeText" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Bool,
                                            op: Op::Call {
                                                func_name: "syntra_file_write_text".to_string(),
                                                args: vec![arg_vals[0], arg_vals[1]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Bool);
                                        return res;
                                    }
                                    "appendText" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Bool,
                                            op: Op::Call {
                                                func_name: "syntra_file_append_text".to_string(),
                                                args: vec![arg_vals[0], arg_vals[1]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Bool);
                                        return res;
                                    }
                                    "exists" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Bool,
                                            op: Op::Call {
                                                func_name: "syntra_file_exists".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Bool);
                                        return res;
                                    }
                                    "delete" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Bool,
                                            op: Op::Call {
                                                func_name: "syntra_file_delete".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Bool);
                                        return res;
                                    }
                                    "size" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Int,
                                            op: Op::Call {
                                                func_name: "syntra_file_size".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Int);
                                        return res;
                                    }
                                    "readLines" => {
                                        let res = self.alloc_val();
                                        let arr_ty = Type::Array(Box::new(Type::String));
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: arr_ty.clone(),
                                            op: Op::Call {
                                                func_name: "syntra_file_read_lines".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, arr_ty);
                                        return res;
                                    }
                                    _ => {}
                                }
                            }

                            // Check for static Sha256 methods
                            if obj_name == "Sha256" && member == "hash" && !arg_vals.is_empty() {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::String,
                                    op: Op::Call {
                                        func_name: "syntra_crypto_sha256".to_string(),
                                        args: vec![arg_vals[0]],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::String);
                                return res;
                            }

                            // Check for static System methods
                            if obj_name == "System" {
                                match member.as_str() {
                                    "args" => {
                                        let res = self.alloc_val();
                                        let arr_ty = Type::Array(Box::new(Type::String));
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: arr_ty.clone(),
                                            op: Op::Call {
                                                func_name: "syntra_system_args".to_string(),
                                                args: vec![],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, arr_ty);
                                        return res;
                                    }
                                    "free" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Void,
                                            op: Op::Call {
                                                func_name: "syntra_free".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Void);
                                        return res;
                                    }
                                    "exit" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Void,
                                            op: Op::Call {
                                                func_name: "syntra_system_exit".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Void);
                                        return res;
                                    }
                                    "exec" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Int,
                                            op: Op::Call {
                                                func_name: "syntra_system_exec".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Int);
                                        return res;
                                    }
                                    "getEnv" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::String,
                                            op: Op::Call {
                                                func_name: "syntra_system_getenv".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::String);
                                        return res;
                                    }
                                    "timeMs" | "currentTimeMillis" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Int,
                                            op: Op::Call {
                                                func_name: "syntra_system_time_ms".to_string(),
                                                args: vec![],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Int);
                                        return res;
                                    }
                                    "println" | "print" => {
                                        let is_ln = member == "println";
                                        for a_val in arg_vals {
                                            let a_ty = self
                                                .value_types
                                                .get(&a_val)
                                                .cloned()
                                                .unwrap_or(Type::String);
                                            let str_val = match a_ty {
                                                Type::String => a_val,
                                                Type::Int if is_ln => {
                                                    let call_res = self.alloc_val();
                                                    self.current_instructions.push(Instruction {
                                                        result: call_res,
                                                        ty: Type::Void,
                                                        op: Op::Call {
                                                            func_name: "syntra_println_int"
                                                                .to_string(),
                                                            args: vec![a_val],
                                                        },
                                                        span: *span,
                                                    });
                                                    self.value_types.insert(call_res, Type::Void);
                                                    continue;
                                                }
                                                Type::Int => {
                                                    let s_res = self.alloc_val();
                                                    self.current_instructions.push(Instruction {
                                                        result: s_res,
                                                        ty: Type::String,
                                                        op: Op::Call {
                                                            func_name: "syntra_int_to_str"
                                                                .to_string(),
                                                            args: vec![a_val],
                                                        },
                                                        span: *span,
                                                    });
                                                    self.value_types.insert(s_res, Type::String);
                                                    s_res
                                                }
                                                Type::Float => {
                                                    let s_res = self.alloc_val();
                                                    self.current_instructions.push(Instruction {
                                                        result: s_res,
                                                        ty: Type::String,
                                                        op: Op::Call {
                                                            func_name: "syntra_float_to_str"
                                                                .to_string(),
                                                            args: vec![a_val],
                                                        },
                                                        span: *span,
                                                    });
                                                    self.value_types.insert(s_res, Type::String);
                                                    s_res
                                                }
                                                Type::Bool => {
                                                    let s_res = self.alloc_val();
                                                    self.current_instructions.push(Instruction {
                                                        result: s_res,
                                                        ty: Type::String,
                                                        op: Op::Call {
                                                            func_name: "syntra_bool_to_str"
                                                                .to_string(),
                                                            args: vec![a_val],
                                                        },
                                                        span: *span,
                                                    });
                                                    self.value_types.insert(s_res, Type::String);
                                                    s_res
                                                }
                                                _ => a_val,
                                            };
                                            let target_fn = if is_ln {
                                                "syntra_println_str"
                                            } else {
                                                "syntra_print_str"
                                            };
                                            let call_res = self.alloc_val();
                                            self.current_instructions.push(Instruction {
                                                result: call_res,
                                                ty: Type::Void,
                                                op: Op::Call {
                                                    func_name: target_fn.to_string(),
                                                    args: vec![str_val],
                                                },
                                                span: *span,
                                            });
                                            self.value_types.insert(call_res, Type::Void);
                                        }
                                        let dummy = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: dummy,
                                            ty: Type::Void,
                                            op: Op::ConstInt(0),
                                            span: *span,
                                        });
                                        self.value_types.insert(dummy, Type::Void);
                                        return dummy;
                                    }
                                    _ => {}
                                }
                            }

                            if obj_name == "String" && member == "fromCharCode" {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::String,
                                    op: Op::Call {
                                        func_name: "syntra_string_from_char_code".to_string(),
                                        args: vec![arg_vals[0]],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::String);
                                return res;
                            }

                            // Check for static Json methods
                            if obj_name == "Json" {
                                match member.as_str() {
                                    "parse" => {
                                        let res = self.alloc_val();
                                        let map_ty = Type::Class("Map".to_string());
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: map_ty.clone(),
                                            op: Op::Call {
                                                func_name: "syntra_json_parse".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, map_ty);
                                        return res;
                                    }
                                    "stringify" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::String,
                                            op: Op::Call {
                                                func_name: "syntra_json_stringify".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::String);
                                        return res;
                                    }
                                    _ => {}
                                }
                            }

                            // Check for static Schema methods
                            if obj_name == "Schema" && member.as_str() == "describe" {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::String,
                                    op: Op::Call {
                                        func_name: "syntra_schema_describe".to_string(),
                                        args: vec![arg_vals[0]],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::String);
                                return res;
                            }

                            // Check for static Task methods
                            if obj_name == "Task" {
                                match member.as_str() {
                                    "spawn" => {
                                        let fn_name = match &args[0] {
                                            Expr::Variable(name, _) => name.clone(),
                                            Expr::Lambda {
                                                params,
                                                body_expr,
                                                body_block,
                                                span,
                                            } => {
                                                let synthetic_name =
                                                    format!("__task_lambda_{}", self.next_val);
                                                let mut lambda_lowerer = Lowerer::new();
                                                lambda_lowerer.class_layouts =
                                                    self.class_layouts.clone();
                                                lambda_lowerer.class_generic_params =
                                                    self.class_generic_params.clone();
                                                lambda_lowerer.function_return_types =
                                                    self.function_return_types.clone();
                                                lambda_lowerer.method_return_types =
                                                    self.method_return_types.clone();
                                                lambda_lowerer.current_class =
                                                    self.current_class.clone();
                                                let mut captured_types =
                                                    self.parent_variable_types.clone();
                                                captured_types.extend(self.variable_types.clone());
                                                lambda_lowerer.parent_variable_types =
                                                    captured_types;

                                                let p_name = params
                                                    .first()
                                                    .map(|p| p.name.clone())
                                                    .unwrap_or_else(|| "_arg".to_string());
                                                let func_decl = FunctionDecl {
                                                    is_async: false,
                                                    name: synthetic_name.clone(),
                                                    generic_params: vec![],
                                                    params: vec![Param {
                                                        name: p_name,
                                                        ty: TypeRef {
                                                            name: "int".to_string(),
                                                            generic_args: vec![],
                                                            is_array: false,
                                                            span: *span,
                                                        },
                                                        span: *span,
                                                    }],
                                                    return_type: Some(TypeRef {
                                                        name: "int".to_string(),
                                                        generic_args: vec![],
                                                        is_array: false,
                                                        span: *span,
                                                    }),
                                                    body: if let Some(b) = body_block {
                                                        b.clone()
                                                    } else if let Some(e) = body_expr {
                                                        Block {
                                                            statements: vec![Stmt::Return {
                                                                value: Some((**e).clone()),
                                                                span: *span,
                                                            }],
                                                            span: *span,
                                                        }
                                                    } else {
                                                        Block {
                                                            statements: vec![],
                                                            span: *span,
                                                        }
                                                    },
                                                    doc: None,
                                                    span: *span,
                                                };
                                                let lowered_func =
                                                    lambda_lowerer.lower_function(&func_decl);
                                                self.extra_functions.push(lowered_func);
                                                self.function_return_types
                                                    .insert(synthetic_name.clone(), Type::Int);
                                                synthetic_name
                                            }
                                            _ => "unknown_fn".to_string(),
                                        };
                                        let fn_ptr = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: fn_ptr,
                                            ty: Type::Int,
                                            op: Op::FuncAddr { func_name: fn_name },
                                            span: *span,
                                        });
                                        self.value_types.insert(fn_ptr, Type::Int);

                                        let user_arg = if arg_vals.len() > 1 {
                                            arg_vals[1]
                                        } else {
                                            let zero = self.alloc_val();
                                            self.current_instructions.push(Instruction {
                                                result: zero,
                                                ty: Type::Int,
                                                op: Op::ConstInt(0),
                                                span: *span,
                                            });
                                            self.value_types.insert(zero, Type::Int);
                                            zero
                                        };

                                        let task_val = self.alloc_val();
                                        let task_ty = Type::Class("Task".to_string());
                                        self.current_instructions.push(Instruction {
                                            result: task_val,
                                            ty: task_ty.clone(),
                                            op: Op::Call {
                                                func_name: "syntra_task_spawn".to_string(),
                                                args: vec![fn_ptr, user_arg],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(task_val, task_ty);
                                        return task_val;
                                    }
                                    "sleep" => {
                                        let res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: res,
                                            ty: Type::Void,
                                            op: Op::Call {
                                                func_name: "syntra_task_sleep".to_string(),
                                                args: vec![arg_vals[0]],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(res, Type::Void);
                                        return res;
                                    }
                                    _ => {}
                                }
                            }

                            if obj_name == "Channel" && member.as_str() == "select" {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::Int,
                                    op: Op::Call {
                                        func_name: "syntra_channel_select".to_string(),
                                        args: vec![arg_vals[0]],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::Int);
                                return res;
                            }

                            if obj_name == "TcpListener" {
                                if member == "serveReactor" {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_http_serve_reactor".to_string(),
                                            args: arg_vals,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                if member == "listen" {
                                    let res = self.alloc_val();
                                    let ret_ty = Type::Class("TcpListener".to_string());
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: ret_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_tcp_listen".to_string(),
                                            args: vec![arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, ret_ty);
                                    return res;
                                }
                            }

                            if obj_name == "TcpStream" {
                                if member == "connect" {
                                    let res = self.alloc_val();
                                    let ret_ty = Type::Class("TcpStream".to_string());
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: ret_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_tcp_connect".to_string(),
                                            args: vec![arg_vals[0], arg_vals[1]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, ret_ty);
                                    return res;
                                }
                                if member == "computeWsAccept" {
                                    let res = self.alloc_val();
                                    let ret_ty = Type::String;
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: ret_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_ws_compute_accept".to_string(),
                                            args: vec![arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, ret_ty);
                                    return res;
                                }
                            }

                            if obj_name == "Arena" {
                                if member == "create" {
                                    let cap_val = if !arg_vals.is_empty() {
                                        arg_vals[0]
                                    } else {
                                        let default_cap = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: default_cap,
                                            ty: Type::Int,
                                            op: Op::ConstInt(65536),
                                            span: *span,
                                        });
                                        self.value_types.insert(default_cap, Type::Int);
                                        default_cap
                                    };
                                    let res = self.alloc_val();
                                    let ret_ty = Type::Class("Arena".to_string());
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: ret_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_arena_create".to_string(),
                                            args: vec![cap_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, ret_ty);
                                    return res;
                                }
                                if member == "scoped" {
                                    let default_cap = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: default_cap,
                                        ty: Type::Int,
                                        op: Op::ConstInt(65536),
                                        span: *span,
                                    });
                                    self.value_types.insert(default_cap, Type::Int);

                                    let arena_val = self.alloc_val();
                                    let arena_ty = Type::Class("Arena".to_string());
                                    self.current_instructions.push(Instruction {
                                        result: arena_val,
                                        ty: arena_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_arena_create".to_string(),
                                            args: vec![default_cap],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(arena_val, arena_ty);

                                    let dummy = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_invoke_fn1".to_string(),
                                            args: vec![arg_vals[0], arena_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy, Type::Int);

                                    let destroy_res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: destroy_res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_arena_destroy".to_string(),
                                            args: vec![arena_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(destroy_res, Type::Void);
                                    return dummy;
                                }
                            }

                            // Check if this is a user-defined namespace function call
                            let ns_fn_name = format!("{obj_name}_{member}");
                            if let Some(ret_ty) =
                                self.function_return_types.get(&ns_fn_name).cloned()
                            {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: ret_ty.clone(),
                                    op: Op::Call {
                                        func_name: ns_fn_name,
                                        args: arg_vals,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, ret_ty);
                                return res;
                            }
                        }

                        let obj_val = self.lower_expression(object);
                        let obj_ty = self.value_types.get(&obj_val).cloned();

                        if obj_ty == Some(Type::String) {
                            match member.as_str() {
                                "length" | "len" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_str_len".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "isEmpty" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Call {
                                            func_name: "syntra_str_is_empty".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                "contains" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Call {
                                            func_name: "syntra_str_contains".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                "indexOf" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_str_index_of".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "startsWith" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Call {
                                            func_name: "syntra_str_starts_with".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                "endsWith" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Call {
                                            func_name: "syntra_str_ends_with".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                "toUpper" | "toUpperCase" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_str_to_upper".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "toLower" | "toLowerCase" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_str_to_lower".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "trim" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_str_trim".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "slice" | "substring" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_str_slice".to_string(),
                                            args: vec![obj_val, arg_vals[0], arg_vals[1]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "charAt" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_str_char_at".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "charCodeAt" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_str_char_code_at".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "replace" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_str_replace".to_string(),
                                            args: vec![obj_val, arg_vals[0], arg_vals[1]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "split" => {
                                    let res = self.alloc_val();
                                    let arr_ty = Type::Array(Box::new(Type::String));
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: arr_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_str_split".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, arr_ty);
                                    return res;
                                }
                                "toInt" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_str_to_int".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if matches!(obj_ty, Some(Type::Array(_))) {
                            match member.as_str() {
                                "length" | "len" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::ArrayLength { array: obj_val },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "isEmpty" => {
                                    let len_val = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: len_val,
                                        ty: Type::Int,
                                        op: Op::ArrayLength { array: obj_val },
                                        span: *span,
                                    });
                                    self.value_types.insert(len_val, Type::Int);

                                    let zero = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: zero,
                                        ty: Type::Int,
                                        op: Op::ConstInt(0),
                                        span: *span,
                                    });
                                    self.value_types.insert(zero, Type::Int);

                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Binary {
                                            op: syntra_syntax::ast::BinaryOp::EqualEqual,
                                            left: len_val,
                                            right: zero,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                "join" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_array_join".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "push" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_array_push".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        let obj_cls = match self.value_types.get(&obj_val) {
                            Some(Type::Class(cls)) => Some(cls.clone()),
                            Some(Type::GenericInstance(cls, _)) => Some(cls.clone()),
                            _ => self.get_object_class(object),
                        };
                        let cls_name = obj_cls.unwrap_or_else(|| "Unknown".to_string());

                        if cls_name == "Map" {
                            match member.as_str() {
                                "set" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_map_set".to_string(),
                                            args: vec![obj_val, arg_vals[0], arg_vals[1]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                "get" => {
                                    let mut ret_ty = Type::Int;
                                    if let Some(Type::GenericInstance(_, gen_args)) =
                                        self.get_object_type(object)
                                    {
                                        if gen_args.len() >= 2 {
                                            ret_ty = gen_args[1].clone();
                                        }
                                    }
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: ret_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_map_get".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, ret_ty);
                                    return res;
                                }
                                "has" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Call {
                                            func_name: "syntra_map_has".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                "remove" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Call {
                                            func_name: "syntra_map_remove".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                "len" | "size" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_map_len".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "keys" => {
                                    let res = self.alloc_val();
                                    let arr_ty = Type::Array(Box::new(Type::String));
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: arr_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_map_keys".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, arr_ty);
                                    return res;
                                }
                                "values" => {
                                    let res = self.alloc_val();
                                    let arr_ty = Type::Array(Box::new(Type::Int));
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: arr_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_map_values".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, arr_ty);
                                    return res;
                                }
                                "clear" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_map_clear".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if cls_name == "Task" {
                            match member.as_str() {
                                "join" | "wait" | "await" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_task_join".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if cls_name == "Channel" {
                            match member.as_str() {
                                "send" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_channel_send".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                "recv" => {
                                    let mut ret_ty = Type::Int;
                                    if let Some(Type::GenericInstance(_, gen_args)) =
                                        self.get_object_type(object)
                                    {
                                        if !gen_args.is_empty() {
                                            ret_ty = gen_args[0].clone();
                                        }
                                    }
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: ret_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_channel_recv".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, ret_ty);
                                    return res;
                                }
                                "close" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_channel_close".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                "len" | "size" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_channel_len".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "isEmpty" => {
                                    let len_res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: len_res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_channel_len".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(len_res, Type::Int);
                                    let zero = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: zero,
                                        ty: Type::Int,
                                        op: Op::ConstInt(0),
                                        span: *span,
                                    });
                                    self.value_types.insert(zero, Type::Int);
                                    let eq_res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: eq_res,
                                        ty: Type::Bool,
                                        op: Op::Binary {
                                            op: BinaryOp::EqualEqual,
                                            left: len_res,
                                            right: zero,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(eq_res, Type::Bool);
                                    return eq_res;
                                }
                                "tryRecv" => {
                                    let mut ret_ty = Type::Int;
                                    if let Some(Type::GenericInstance(_, gen_args)) =
                                        self.get_object_type(object)
                                    {
                                        if !gen_args.is_empty() {
                                            ret_ty = gen_args[0].clone();
                                        }
                                    }
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: ret_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_channel_try_recv".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, ret_ty);
                                    return res;
                                }
                                "trySend" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Bool,
                                        op: Op::Call {
                                            func_name: "syntra_channel_try_send".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Bool);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if cls_name == "ThreadPool" {
                            match member.as_str() {
                                "submit" => {
                                    let fn_name = match &args[0] {
                                        Expr::Variable(name, _) => name.clone(),
                                        Expr::Lambda {
                                            params,
                                            body_expr,
                                            body_block,
                                            span,
                                        } => {
                                            let synthetic_name =
                                                format!("__pool_lambda_{}", self.next_val);
                                            let mut lambda_lowerer = Lowerer::new();
                                            lambda_lowerer.class_layouts =
                                                self.class_layouts.clone();
                                            lambda_lowerer.class_generic_params =
                                                self.class_generic_params.clone();
                                            lambda_lowerer.function_return_types =
                                                self.function_return_types.clone();
                                            lambda_lowerer.method_return_types =
                                                self.method_return_types.clone();
                                            lambda_lowerer.current_class =
                                                self.current_class.clone();
                                            let mut captured_types =
                                                self.parent_variable_types.clone();
                                            captured_types.extend(self.variable_types.clone());
                                            lambda_lowerer.parent_variable_types = captured_types;

                                            let p_name = params
                                                .first()
                                                .map(|p| p.name.clone())
                                                .unwrap_or_else(|| "_arg".to_string());
                                            let func_decl = FunctionDecl {
                                                is_async: false,
                                                name: synthetic_name.clone(),
                                                generic_params: vec![],
                                                params: vec![Param {
                                                    name: p_name,
                                                    ty: TypeRef {
                                                        name: "int".to_string(),
                                                        generic_args: vec![],
                                                        is_array: false,
                                                        span: *span,
                                                    },
                                                    span: *span,
                                                }],
                                                return_type: Some(TypeRef {
                                                    name: "int".to_string(),
                                                    generic_args: vec![],
                                                    is_array: false,
                                                    span: *span,
                                                }),
                                                body: if let Some(b) = body_block {
                                                    b.clone()
                                                } else if let Some(e) = body_expr {
                                                    Block {
                                                        statements: vec![Stmt::Return {
                                                            value: Some((**e).clone()),
                                                            span: *span,
                                                        }],
                                                        span: *span,
                                                    }
                                                } else {
                                                    Block {
                                                        statements: vec![],
                                                        span: *span,
                                                    }
                                                },
                                                doc: None,
                                                span: *span,
                                            };
                                            let lowered_func =
                                                lambda_lowerer.lower_function(&func_decl);
                                            self.extra_functions.push(lowered_func);
                                            self.function_return_types
                                                .insert(synthetic_name.clone(), Type::Int);
                                            synthetic_name
                                        }
                                        _ => "unknown_fn".to_string(),
                                    };
                                    let fn_ptr = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: fn_ptr,
                                        ty: Type::Int,
                                        op: Op::FuncAddr { func_name: fn_name },
                                        span: *span,
                                    });
                                    self.value_types.insert(fn_ptr, Type::Int);

                                    let user_arg = if arg_vals.len() > 1 {
                                        arg_vals[1]
                                    } else {
                                        let zero = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: zero,
                                            ty: Type::Int,
                                            op: Op::ConstInt(0),
                                            span: *span,
                                        });
                                        self.value_types.insert(zero, Type::Int);
                                        zero
                                    };

                                    let task_val = self.alloc_val();
                                    let task_ty = Type::Class("Task".to_string());
                                    self.current_instructions.push(Instruction {
                                        result: task_val,
                                        ty: task_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_threadpool_submit".to_string(),
                                            args: vec![obj_val, fn_ptr, user_arg],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(task_val, task_ty);
                                    return task_val;
                                }
                                "shutdown" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_threadpool_shutdown".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if cls_name == "TcpListener" {
                            match member.as_str() {
                                "accept" => {
                                    let res = self.alloc_val();
                                    let stream_ty = Type::Class("TcpStream".to_string());
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: stream_ty.clone(),
                                        op: Op::Call {
                                            func_name: "syntra_tcp_accept".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, stream_ty);
                                    return res;
                                }
                                "port" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_listener_port".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "close" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_close".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if cls_name == "TcpStream" {
                            match member.as_str() {
                                "read" => {
                                    let max_len_val = if !arg_vals.is_empty() {
                                        arg_vals[0]
                                    } else {
                                        let default_max = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: default_max,
                                            ty: Type::Int,
                                            op: Op::ConstInt(65536),
                                            span: *span,
                                        });
                                        self.value_types.insert(default_max, Type::Int);
                                        default_max
                                    };
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_read".to_string(),
                                            args: vec![obj_val, max_len_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "write" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_write".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "close" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_close".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                "readWsFrame" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_read_ws_frame".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::String);
                                    return res;
                                }
                                "writeWsFrame" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_write_ws_frame".to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                "writeWsClientFrame" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_tcp_write_ws_client_frame"
                                                .to_string(),
                                            args: vec![obj_val, arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if cls_name == "Arena" {
                            match member.as_str() {
                                "alloc" => {
                                    let obj_to_alloc = arg_vals[0];
                                    let size_val = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: size_val,
                                        ty: Type::Int,
                                        op: Op::ConstInt(64),
                                        span: *span,
                                    });
                                    self.value_types.insert(size_val, Type::Int);

                                    let alloc_call = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: alloc_call,
                                        ty: Type::Class("Arena".to_string()),
                                        op: Op::Call {
                                            func_name: "syntra_arena_alloc".to_string(),
                                            args: vec![obj_val, size_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types
                                        .insert(alloc_call, Type::Class("Arena".to_string()));

                                    return obj_to_alloc;
                                }
                                "reset" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_arena_reset".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                "destroy" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Void,
                                        op: Op::Call {
                                            func_name: "syntra_arena_destroy".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Void);
                                    return res;
                                }
                                "allocatedBytes" => {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_arena_bytes_allocated".to_string(),
                                            args: vec![obj_val],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                _ => {}
                            }
                        }

                        if cls_name == "Route" {
                            if member == "with" {
                                let m_list = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: m_list,
                                    ty: Type::Array(Box::new(Type::Unknown)),
                                    op: Op::GetField {
                                        object: obj_val,
                                        field_name: "middlewaresList".to_string(),
                                        offset: 104,
                                    },
                                    span: *span,
                                });
                                self.value_types
                                    .insert(m_list, Type::Array(Box::new(Type::Unknown)));

                                for arg in arg_vals {
                                    let dummy = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: "syntra_array_push".to_string(),
                                            args: vec![m_list, arg],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy, Type::Int);
                                }
                                return obj_val;
                            }
                            if member == "tag" {
                                let dummy = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: dummy,
                                    ty: Type::Void,
                                    op: Op::SetField {
                                        object: obj_val,
                                        field_name: "routeTag".to_string(),
                                        offset: 32,
                                        val: arg_vals[0],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(dummy, Type::Void);
                                return obj_val;
                            }
                            if member == "summary" {
                                let dummy = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: dummy,
                                    ty: Type::Void,
                                    op: Op::SetField {
                                        object: obj_val,
                                        field_name: "routeSummary".to_string(),
                                        offset: 40,
                                        val: arg_vals[0],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(dummy, Type::Void);
                                return obj_val;
                            }
                            if member == "description" {
                                let dummy = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: dummy,
                                    ty: Type::Void,
                                    op: Op::SetField {
                                        object: obj_val,
                                        field_name: "routeDescription".to_string(),
                                        offset: 48,
                                        val: arg_vals[0],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(dummy, Type::Void);
                                return obj_val;
                            }
                            if member == "request" || member == "body" {
                                let dummy1 = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: dummy1,
                                    ty: Type::Void,
                                    op: Op::SetField {
                                        object: obj_val,
                                        field_name: "requestSchemaName".to_string(),
                                        offset: 56,
                                        val: arg_vals[0],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(dummy1, Type::Void);

                                let schema_json = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: schema_json,
                                    ty: Type::String,
                                    op: Op::Call {
                                        func_name: "syntra_schema_describe".to_string(),
                                        args: vec![arg_vals[0]],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(schema_json, Type::String);

                                let dummy2 = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: dummy2,
                                    ty: Type::Void,
                                    op: Op::SetField {
                                        object: obj_val,
                                        field_name: "requestSchemaJson".to_string(),
                                        offset: 64,
                                        val: schema_json,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(dummy2, Type::Void);
                                return obj_val;
                            }
                            if member == "response" {
                                if arg_vals.len() == 2 {
                                    let dummy1 = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy1,
                                        ty: Type::Void,
                                        op: Op::SetField {
                                            object: obj_val,
                                            field_name: "responseStatusCode".to_string(),
                                            offset: 72,
                                            val: arg_vals[0],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy1, Type::Void);

                                    let dummy2 = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy2,
                                        ty: Type::Void,
                                        op: Op::SetField {
                                            object: obj_val,
                                            field_name: "responseDescription".to_string(),
                                            offset: 96,
                                            val: arg_vals[1],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy2, Type::Void);
                                } else if arg_vals.len() >= 3 {
                                    let dummy1 = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy1,
                                        ty: Type::Void,
                                        op: Op::SetField {
                                            object: obj_val,
                                            field_name: "responseStatusCode".to_string(),
                                            offset: 72,
                                            val: arg_vals[0],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy1, Type::Void);

                                    let dummy2 = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy2,
                                        ty: Type::Void,
                                        op: Op::SetField {
                                            object: obj_val,
                                            field_name: "responseSchemaName".to_string(),
                                            offset: 80,
                                            val: arg_vals[1],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy2, Type::Void);

                                    let schema_json = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: schema_json,
                                        ty: Type::String,
                                        op: Op::Call {
                                            func_name: "syntra_schema_describe".to_string(),
                                            args: vec![arg_vals[1]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(schema_json, Type::String);

                                    let dummy3 = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy3,
                                        ty: Type::Void,
                                        op: Op::SetField {
                                            object: obj_val,
                                            field_name: "responseSchemaJson".to_string(),
                                            offset: 88,
                                            val: schema_json,
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy3, Type::Void);

                                    let dummy4 = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: dummy4,
                                        ty: Type::Void,
                                        op: Op::SetField {
                                            object: obj_val,
                                            field_name: "responseDescription".to_string(),
                                            offset: 96,
                                            val: arg_vals[2],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(dummy4, Type::Void);
                                }
                                return obj_val;
                            }
                        }

                        if !self
                            .method_return_types
                            .contains_key(&(cls_name.clone(), member.clone()))
                        {
                            let field_info = self
                                .class_layouts
                                .get(&cls_name)
                                .and_then(|l| l.fields.get(member))
                                .map(|f| (f.offset, f.ty.clone()));

                            if let Some((offset, f_ty)) = field_info {
                                let fn_ptr = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: fn_ptr,
                                    ty: f_ty.clone(),
                                    op: Op::GetField {
                                        object: obj_val,
                                        field_name: member.clone(),
                                        offset,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(fn_ptr, f_ty);

                                let invoke_symbol = match arg_vals.len() {
                                    0 => "syntra_invoke_fn0",
                                    1 => "syntra_invoke_fn1",
                                    2 => "syntra_invoke_fn2",
                                    _ => "syntra_invoke_fn1",
                                };
                                let mut call_args = vec![fn_ptr];
                                call_args.extend(arg_vals);
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::Int,
                                    op: Op::Call {
                                        func_name: invoke_symbol.to_string(),
                                        args: call_args,
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::Int);
                                return res;
                            }
                        }

                        let mangled_fn = format!("{cls_name}_{member}");

                        let mut full_args = vec![obj_val];
                        full_args.extend(arg_vals);

                        let mut ret_ty = self
                            .method_return_types
                            .get(&(cls_name.clone(), member.clone()))
                            .cloned()
                            .unwrap_or(Type::Void);

                        if let Some(Type::GenericInstance(_, gen_args)) =
                            self.get_object_type(object)
                        {
                            if let Some(params) = self.class_generic_params.get(&cls_name) {
                                if let Type::GenericParam(p_name) | Type::Class(p_name) = &ret_ty {
                                    if let Some(idx) = params.iter().position(|p| p == p_name) {
                                        if let Some(concrete_ty) = gen_args.get(idx) {
                                            ret_ty = concrete_ty.clone();
                                        }
                                    }
                                }
                            }
                        }

                        let res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: ret_ty.clone(),
                            op: Op::Call {
                                func_name: mangled_fn,
                                args: full_args,
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, ret_ty);
                        res
                    }
                    Expr::Variable(name, _) => {
                        // Built-in functions with multi-arg folding support
                        if name == "min" || name == "max" {
                            let fn_symbol = if name == "min" {
                                "syntra_math_min"
                            } else {
                                "syntra_math_max"
                            };
                            let arr_fn_symbol = if name == "min" {
                                "syntra_array_min"
                            } else {
                                "syntra_array_max"
                            };

                            if arg_vals.is_empty() {
                                let res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: res,
                                    ty: Type::Int,
                                    op: Op::ConstInt(0),
                                    span: *span,
                                });
                                self.value_types.insert(res, Type::Int);
                                return res;
                            }

                            if arg_vals.len() == 1 {
                                let first_ty = self.value_types.get(&arg_vals[0]);
                                if matches!(first_ty, Some(Type::Array(_))) {
                                    let res = self.alloc_val();
                                    self.current_instructions.push(Instruction {
                                        result: res,
                                        ty: Type::Int,
                                        op: Op::Call {
                                            func_name: arr_fn_symbol.to_string(),
                                            args: vec![arg_vals[0]],
                                        },
                                        span: *span,
                                    });
                                    self.value_types.insert(res, Type::Int);
                                    return res;
                                }
                                return arg_vals[0];
                            }

                            let mut curr = arg_vals[0];
                            for next_val in &arg_vals[1..] {
                                let fold_res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: fold_res,
                                    ty: Type::Int,
                                    op: Op::Call {
                                        func_name: fn_symbol.to_string(),
                                        args: vec![curr, *next_val],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(fold_res, Type::Int);
                                curr = fold_res;
                            }
                            return curr;
                        }

                        if name == "abs" {
                            let arg = if let Some(v) = arg_vals.first() {
                                *v
                            } else {
                                let zero = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: zero,
                                    ty: Type::Int,
                                    op: Op::ConstInt(0),
                                    span: *span,
                                });
                                self.value_types.insert(zero, Type::Int);
                                zero
                            };
                            let res = self.alloc_val();
                            self.current_instructions.push(Instruction {
                                result: res,
                                ty: Type::Int,
                                op: Op::Call {
                                    func_name: "syntra_math_abs".to_string(),
                                    args: vec![arg],
                                },
                                span: *span,
                            });
                            self.value_types.insert(res, Type::Int);
                            return res;
                        }

                        // Built-in print / println with type-safe formatting
                        if name == "println" || name == "print" {
                            let is_ln = name == "println";
                            for a_val in arg_vals {
                                let a_ty = self
                                    .value_types
                                    .get(&a_val)
                                    .cloned()
                                    .unwrap_or(Type::String);
                                let str_val = match a_ty {
                                    Type::String => a_val,
                                    Type::Int if is_ln => {
                                        let call_res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: call_res,
                                            ty: Type::Void,
                                            op: Op::Call {
                                                func_name: "syntra_println_int".to_string(),
                                                args: vec![a_val],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(call_res, Type::Void);
                                        continue;
                                    }
                                    Type::Int => {
                                        let s_res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: s_res,
                                            ty: Type::String,
                                            op: Op::Call {
                                                func_name: "syntra_int_to_str".to_string(),
                                                args: vec![a_val],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(s_res, Type::String);
                                        s_res
                                    }
                                    Type::Bool => {
                                        let s_res = self.alloc_val();
                                        self.current_instructions.push(Instruction {
                                            result: s_res,
                                            ty: Type::String,
                                            op: Op::Call {
                                                func_name: "syntra_bool_to_str".to_string(),
                                                args: vec![a_val],
                                            },
                                            span: *span,
                                        });
                                        self.value_types.insert(s_res, Type::String);
                                        s_res
                                    }
                                    _ => a_val,
                                };

                                let target_fn = if is_ln {
                                    "syntra_println_str"
                                } else {
                                    "syntra_print_str"
                                };
                                let call_res = self.alloc_val();
                                self.current_instructions.push(Instruction {
                                    result: call_res,
                                    ty: Type::Void,
                                    op: Op::Call {
                                        func_name: target_fn.to_string(),
                                        args: vec![str_val],
                                    },
                                    span: *span,
                                });
                                self.value_types.insert(call_res, Type::Void);
                            }
                            let res = self.alloc_val();
                            self.current_instructions.push(Instruction {
                                result: res,
                                ty: Type::Void,
                                op: Op::ConstInt(0),
                                span: *span,
                            });
                            self.value_types.insert(res, Type::Void);
                            return res;
                        }

                        if self.variables.contains_key(name)
                            && !self.function_return_types.contains_key(name)
                        {
                            let fn_ptr = self.alloc_val();
                            let slot = self.variables[name];
                            let var_ty =
                                self.variable_types.get(name).cloned().unwrap_or(Type::Int);
                            self.current_instructions.push(Instruction {
                                result: fn_ptr,
                                ty: var_ty.clone(),
                                op: Op::Load { ptr: slot },
                                span: *span,
                            });
                            self.value_types.insert(fn_ptr, var_ty);

                            let invoke_symbol = match arg_vals.len() {
                                0 => "syntra_invoke_fn0",
                                1 => "syntra_invoke_fn1",
                                2 => "syntra_invoke_fn2",
                                _ => "syntra_invoke_fn1",
                            };
                            let mut call_args = vec![fn_ptr];
                            call_args.extend(arg_vals);
                            let res = self.alloc_val();
                            self.current_instructions.push(Instruction {
                                result: res,
                                ty: Type::Int,
                                op: Op::Call {
                                    func_name: invoke_symbol.to_string(),
                                    args: call_args,
                                },
                                span: *span,
                            });
                            self.value_types.insert(res, Type::Int);
                            return res;
                        }

                        let ret_ty = self
                            .function_return_types
                            .get(name)
                            .cloned()
                            .unwrap_or(Type::Int);

                        let res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: ret_ty.clone(),
                            op: Op::Call {
                                func_name: name.clone(),
                                args: arg_vals,
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, ret_ty);
                        res
                    }
                    _ => {
                        let fn_ptr = self.lower_expression(callee);
                        let invoke_symbol = match arg_vals.len() {
                            0 => "syntra_invoke_fn0",
                            1 => "syntra_invoke_fn1",
                            2 => "syntra_invoke_fn2",
                            _ => "syntra_invoke_fn1",
                        };
                        let mut call_args = vec![fn_ptr];
                        call_args.extend(arg_vals);
                        let res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: Type::Int,
                            op: Op::Call {
                                func_name: invoke_symbol.to_string(),
                                args: call_args,
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, Type::Int);
                        res
                    }
                }
            }
            Expr::New {
                class_name,
                generic_args: _,
                args,
                span,
            } => {
                if class_name == "Map" {
                    let res = self.alloc_val();
                    let class_type = Type::Class("Map".to_string());
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: class_type.clone(),
                        op: Op::Call {
                            func_name: "syntra_map_new".to_string(),
                            args: Vec::new(),
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, class_type);
                    return res;
                }
                if class_name == "Channel" {
                    let cap_val = if !args.is_empty() {
                        self.lower_expression(&args[0])
                    } else {
                        let zero = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: zero,
                            ty: Type::Int,
                            op: Op::ConstInt(0),
                            span: *span,
                        });
                        self.value_types.insert(zero, Type::Int);
                        zero
                    };
                    let res = self.alloc_val();
                    let class_type = Type::Class("Channel".to_string());
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: class_type.clone(),
                        op: Op::Call {
                            func_name: "syntra_channel_new".to_string(),
                            args: vec![cap_val],
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, class_type);
                    return res;
                }
                if class_name == "ThreadPool" {
                    let workers_val = if !args.is_empty() {
                        self.lower_expression(&args[0])
                    } else {
                        let four = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: four,
                            ty: Type::Int,
                            op: Op::ConstInt(4),
                            span: *span,
                        });
                        self.value_types.insert(four, Type::Int);
                        four
                    };
                    let res = self.alloc_val();
                    let class_type = Type::Class("ThreadPool".to_string());
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: class_type.clone(),
                        op: Op::Call {
                            func_name: "syntra_threadpool_new".to_string(),
                            args: vec![workers_val],
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, class_type);
                    return res;
                }
                if class_name == "Arena" {
                    let cap_val = if !args.is_empty() {
                        self.lower_expression(&args[0])
                    } else {
                        let default_cap = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: default_cap,
                            ty: Type::Int,
                            op: Op::ConstInt(65536),
                            span: *span,
                        });
                        self.value_types.insert(default_cap, Type::Int);
                        default_cap
                    };
                    let res = self.alloc_val();
                    let class_type = Type::Class("Arena".to_string());
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: class_type.clone(),
                        op: Op::Call {
                            func_name: "syntra_arena_create".to_string(),
                            args: vec![cap_val],
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, class_type);
                    return res;
                }
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.lower_expression(a));
                }
                let res = self.alloc_val();
                let class_type = Type::Class(class_name.clone());
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: class_type.clone(),
                    op: Op::Call {
                        func_name: format!("{class_name}_new"),
                        args: arg_vals,
                    },
                    span: *span,
                });
                self.value_types.insert(res, class_type);
                res
            }
            Expr::MemberAccess {
                object,
                member,
                span,
            } => {
                let obj_val = self.lower_expression(object);
                let obj_cls = match self.value_types.get(&obj_val) {
                    Some(Type::Class(cls)) => Some(cls.clone()),
                    Some(Type::GenericInstance(cls, _)) => Some(cls.clone()),
                    _ => self.get_object_class(object),
                };

                let is_class_field = if let Some(ref cls_name) = obj_cls {
                    self.class_layouts
                        .get(cls_name)
                        .map(|l| l.fields.contains_key(member))
                        .unwrap_or(false)
                } else {
                    false
                };

                if !is_class_field && (member == "length" || member == "len" || member == "size") {
                    let obj_ty = self.value_types.get(&obj_val).cloned();
                    let res = self.alloc_val();
                    if obj_ty == Some(Type::String) {
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: Type::Int,
                            op: Op::Call {
                                func_name: "syntra_str_len".to_string(),
                                args: vec![obj_val],
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, Type::Int);
                        return res;
                    } else if matches!(obj_ty, Some(Type::Class(ref name)) if name == "Map")
                        || matches!(obj_ty, Some(Type::GenericInstance(ref name, _)) if name == "Map")
                    {
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: Type::Int,
                            op: Op::Call {
                                func_name: "syntra_map_len".to_string(),
                                args: vec![obj_val],
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, Type::Int);
                        return res;
                    } else {
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: Type::Int,
                            op: Op::ArrayLength { array: obj_val },
                            span: *span,
                        });
                        self.value_types.insert(res, Type::Int);
                        return res;
                    }
                }
                if let Some(cls_name) = obj_cls {
                    let field_info = self
                        .class_layouts
                        .get(&cls_name)
                        .and_then(|l| l.fields.get(member))
                        .map(|f| (f.offset, f.ty.clone()));

                    if let Some((offset, f_ty)) = field_info {
                        let res = self.alloc_val();
                        self.current_instructions.push(Instruction {
                            result: res,
                            ty: f_ty.clone(),
                            op: Op::GetField {
                                object: obj_val,
                                field_name: member.clone(),
                                offset,
                            },
                            span: *span,
                        });
                        self.value_types.insert(res, f_ty);
                        return res;
                    }
                }
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Int,
                    op: Op::ConstInt(0),
                    span: *span,
                });
                self.value_types.insert(res, Type::Int);
                res
            }
            Expr::ArrayLit { elements, span } => {
                let len = elements.len() as i64;
                let len_val = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: len_val,
                    ty: Type::Int,
                    op: Op::ConstInt(len),
                    span: *span,
                });
                self.value_types.insert(len_val, Type::Int);

                let mut elem_vals = Vec::new();
                for elem in elements {
                    let v = self.lower_expression(elem);
                    elem_vals.push((elem.span(), v));
                }

                let elem_ty = if let Some((_, first_val)) = elem_vals.first() {
                    self.value_types
                        .get(first_val)
                        .cloned()
                        .unwrap_or(Type::Int)
                } else {
                    Type::Int
                };

                let arr_val = self.alloc_val();
                let arr_ty = Type::Array(Box::new(elem_ty.clone()));
                self.current_instructions.push(Instruction {
                    result: arr_val,
                    ty: arr_ty.clone(),
                    op: Op::AllocArray {
                        elem_ty: elem_ty.clone(),
                        len: len_val,
                    },
                    span: *span,
                });
                self.value_types.insert(arr_val, arr_ty);

                for (idx, (elem_span, elem_val)) in elem_vals.into_iter().enumerate() {
                    let idx_val = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: idx_val,
                        ty: Type::Int,
                        op: Op::ConstInt(idx as i64),
                        span: elem_span,
                    });
                    self.value_types.insert(idx_val, Type::Int);

                    let set_res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: set_res,
                        ty: Type::Void,
                        op: Op::SetIndex {
                            array: arr_val,
                            index: idx_val,
                            val: elem_val,
                        },
                        span: elem_span,
                    });
                    self.value_types.insert(set_res, Type::Void);
                }

                arr_val
            }
            Expr::Index { array, index, span } => {
                let arr_val = self.lower_expression(array);
                let idx_val = self.lower_expression(index);
                let is_map = match self.value_types.get(&arr_val) {
                    Some(Type::Class(c)) if c == "Map" => true,
                    Some(Type::GenericInstance(c, _)) if c == "Map" => true,
                    _ => {
                        if let Expr::Variable(arr_name, _) = array.as_ref() {
                            matches!(self.variable_types.get(arr_name), Some(Type::Class(c)) | Some(Type::GenericInstance(c, _)) if c == "Map")
                        } else {
                            false
                        }
                    }
                };
                if is_map {
                    let mut ret_ty = Type::Int;
                    if let Some(Type::GenericInstance(_, gen_args)) = self.get_object_type(array) {
                        if gen_args.len() >= 2 {
                            ret_ty = gen_args[1].clone();
                        }
                    }
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: ret_ty.clone(),
                        op: Op::Call {
                            func_name: "syntra_map_get".to_string(),
                            args: vec![arr_val, idx_val],
                        },
                        span: *span,
                    });
                    self.value_types.insert(res, ret_ty);
                    return res;
                }
                let elem_ty = match self.value_types.get(&arr_val) {
                    Some(Type::Array(inner)) => (**inner).clone(),
                    _ => {
                        if let Expr::Variable(arr_name, _) = array.as_ref() {
                            match self.variable_types.get(arr_name) {
                                Some(Type::Array(inner)) => (**inner).clone(),
                                _ => Type::Int,
                            }
                        } else {
                            Type::Int
                        }
                    }
                };
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: elem_ty.clone(),
                    op: Op::GetIndex {
                        array: arr_val,
                        index: idx_val,
                    },
                    span: *span,
                });
                self.value_types.insert(res, elem_ty);
                res
            }
            Expr::Lambda {
                params,
                body_expr,
                body_block,
                span,
            } => {
                let synthetic_name = format!("__user_lambda_{}", self.next_val);
                self.next_val += 1;
                let mut lambda_lowerer = Lowerer::new();
                lambda_lowerer.class_layouts = self.class_layouts.clone();
                lambda_lowerer.class_generic_params = self.class_generic_params.clone();
                lambda_lowerer.function_return_types = self.function_return_types.clone();
                lambda_lowerer.method_return_types = self.method_return_types.clone();
                lambda_lowerer.current_class = self.current_class.clone();
                let mut captured_types = self.parent_variable_types.clone();
                captured_types.extend(self.variable_types.clone());
                lambda_lowerer.parent_variable_types = captured_types;

                let mut fn_params = Vec::new();
                for p in params {
                    fn_params.push(Param {
                        name: p.name.clone(),
                        ty: p.ty.clone(),
                        span: p.span,
                    });
                }
                let body_stmts = if let Some(blk) = body_block {
                    blk.statements.clone()
                } else if let Some(e) = body_expr {
                    vec![Stmt::Return {
                        value: Some((**e).clone()),
                        span: *span,
                    }]
                } else {
                    vec![]
                };

                let has_return_val = body_expr.is_some()
                    || body_stmts
                        .iter()
                        .any(|s| matches!(s, Stmt::Return { value: Some(_), .. }));

                let func_decl = FunctionDecl {
                    is_async: false,
                    name: synthetic_name.clone(),
                    generic_params: vec![],
                    params: fn_params,
                    return_type: if has_return_val {
                        Some(syntra_syntax::TypeRef {
                            name: "int".to_string(),
                            generic_args: vec![],
                            is_array: false,
                            span: *span,
                        })
                    } else {
                        None
                    },
                    body: syntra_syntax::Block {
                        statements: body_stmts,
                        span: *span,
                    },
                    doc: None,
                    span: *span,
                };
                let lowered_func = lambda_lowerer.lower_function(&func_decl);
                self.extra_functions.push(lowered_func);
                self.extra_functions.extend(lambda_lowerer.extra_functions);
                self.function_return_types.insert(
                    synthetic_name.clone(),
                    if has_return_val {
                        Type::Int
                    } else {
                        Type::Void
                    },
                );

                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Int,
                    op: Op::FuncAddr {
                        func_name: synthetic_name,
                    },
                    span: *span,
                });
                self.value_types.insert(res, Type::Int);
                res
            }
            Expr::Await { expr, span } => {
                let inner_val = self.lower_expression(expr);
                let inner_ty = self
                    .value_types
                    .get(&inner_val)
                    .cloned()
                    .unwrap_or(Type::Unknown);
                let result_ty = match inner_ty {
                    Type::GenericInstance(ref name, ref args)
                        if name == "Task" && !args.is_empty() =>
                    {
                        args[0].clone()
                    }
                    _ => Type::Int,
                };
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: result_ty.clone(),
                    op: Op::Call {
                        func_name: "syntra_task_join".to_string(),
                        args: vec![inner_val],
                    },
                    span: *span,
                });
                self.value_types.insert(res, result_ty);
                res
            }
        }
    }
}
