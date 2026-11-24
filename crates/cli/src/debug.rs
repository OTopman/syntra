use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use crate::lsp::{JsonParser, JsonValue};
use syntra_runtime::{
    create_debug_channel, DebugCommand, DebugEvent, DebugHandle, Interpreter, PauseReason, Value,
};

/// Runs an interactive terminal debugging session (syn debug file.syn)
pub fn run_cli_debugger(file_path: &str, args: Vec<String>) -> Result<(), String> {
    println!("===============================================================");
    println!("          Syntra Interactive Debugger (syn-dbg) v0.1.0        ");
    println!("===============================================================");
    println!("Target Source: {file_path}");
    println!("Type 'help' or 'h' for a list of debugger commands.");
    println!("===============================================================");

    let path = Path::new(file_path);
    if let Err(mismatches) = crate::package::verify_package_consumption(path) {
        let mut err =
            String::from("Dependency verification failed at debugger consumption time:\n");
        for m in mismatches {
            err.push_str(&format!("   • {m}\n"));
        }
        err.push_str("\nRefusing to debug tampered dependencies. Run 'syn install' or investigate tampering.");
        return Err(err);
    }

    let mut loader = syntra_frontend::loader::ModuleLoader::new();
    let program = loader
        .load_program(path)
        .map_err(|diags| format!("Failed to load program: {diags:?}"))?;

    let mut sources: HashMap<String, String> = HashMap::new();
    for (p, src) in &loader.sources {
        sources.insert(p.display().to_string(), src.clone());
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            sources.insert(name.to_string(), src.clone());
        }
    }
    if let Ok(src) = std::fs::read_to_string(file_path) {
        sources.insert(file_path.to_string(), src.clone());
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            sources.insert(name.to_string(), src);
        }
    }

    let (debug_state, handle) = create_debug_channel();

    let sources_worker = sources.clone();
    let file_worker = file_path.to_string();
    let prog_worker = program;
    let args_worker = args;

    let worker = thread::spawn(move || {
        let mut interp = Interpreter::new();
        interp.set_args(args_worker);
        interp.attach_debugger(debug_state);
        for (f, s) in sources_worker {
            interp.set_source(&f, &s);
        }
        interp.current_file = file_worker;
        interp.run_program(&prog_worker)
    });

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    // Wait for initial entry pause
    let (mut current_file, mut current_line, mut is_running) = match handle.recv_event() {
        Ok(DebugEvent::Paused {
            reason,
            file,
            line,
            fn_name,
        }) => {
            print_pause_banner(&reason, &file, line, &fn_name, &sources);
            (file, line, true)
        }
        Ok(DebugEvent::Terminated { exit_code }) => {
            println!("\n[Program exited immediately with code: {exit_code}]");
            (file_path.to_string(), 1, false)
        }
        _ => (file_path.to_string(), 1, true),
    };

    let mut line_buf = String::new();

    while is_running {
        print!("(syn-dbg) ");
        io::stdout().flush().unwrap();
        line_buf.clear();

        if stdin_lock.read_line(&mut line_buf).unwrap_or(0) == 0 {
            break;
        }

        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = parts[0];

        match cmd {
            "h" | "help" => {
                print_help();
            }
            "b" | "break" => {
                if parts.len() < 2 {
                    println!("Usage: break <line> or break <file>:<line>");
                    continue;
                }
                let target = parts[1];
                let (f, l) = if target.contains(':') {
                    let mut split = target.split(':');
                    let file = split.next().unwrap_or(&current_file);
                    let line_num: usize = split
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(current_line);
                    (file.to_string(), line_num)
                } else {
                    let line_num: usize = target.parse().unwrap_or(current_line);
                    (current_file.clone(), line_num)
                };

                let _ = handle.send(DebugCommand::SetBreakpoint {
                    file: f.clone(),
                    line: l,
                });
                println!("Breakpoint set at {f}:{l}");
            }
            "d" | "delete" | "clear" => {
                if parts.len() < 2 {
                    println!("Usage: delete <line> or delete <file>:<line>");
                    continue;
                }
                let target = parts[1];
                let (f, l) = if target.contains(':') {
                    let mut split = target.split(':');
                    let file = split.next().unwrap_or(&current_file);
                    let line_num: usize = split
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(current_line);
                    (file.to_string(), line_num)
                } else {
                    let line_num: usize = target.parse().unwrap_or(current_line);
                    (current_file.clone(), line_num)
                };

                let _ = handle.send(DebugCommand::RemoveBreakpoint {
                    file: f.clone(),
                    line: l,
                });
                println!("Breakpoint removed from {f}:{l}");
            }
            "c" | "continue" => {
                let _ = handle.send(DebugCommand::Continue);
                match handle.recv_event() {
                    Ok(DebugEvent::Paused {
                        reason,
                        file,
                        line,
                        fn_name,
                    }) => {
                        current_file = file;
                        current_line = line;
                        print_pause_banner(
                            &reason,
                            &current_file,
                            current_line,
                            &fn_name,
                            &sources,
                        );
                    }
                    Ok(DebugEvent::Terminated { exit_code }) => {
                        println!("\n[Process finished with exit code: {exit_code}]");
                        is_running = false;
                    }
                    _ => break,
                }
            }
            "s" | "step" => {
                let _ = handle.send(DebugCommand::StepInto);
                match handle.recv_event() {
                    Ok(DebugEvent::Paused {
                        reason,
                        file,
                        line,
                        fn_name,
                    }) => {
                        current_file = file;
                        current_line = line;
                        print_pause_banner(
                            &reason,
                            &current_file,
                            current_line,
                            &fn_name,
                            &sources,
                        );
                    }
                    Ok(DebugEvent::Terminated { exit_code }) => {
                        println!("\n[Process finished with exit code: {exit_code}]");
                        is_running = false;
                    }
                    _ => break,
                }
            }
            "n" | "next" => {
                let _ = handle.send(DebugCommand::StepOver);
                match handle.recv_event() {
                    Ok(DebugEvent::Paused {
                        reason,
                        file,
                        line,
                        fn_name,
                    }) => {
                        current_file = file;
                        current_line = line;
                        print_pause_banner(
                            &reason,
                            &current_file,
                            current_line,
                            &fn_name,
                            &sources,
                        );
                    }
                    Ok(DebugEvent::Terminated { exit_code }) => {
                        println!("\n[Process finished with exit code: {exit_code}]");
                        is_running = false;
                    }
                    _ => break,
                }
            }
            "fin" | "finish" => {
                let _ = handle.send(DebugCommand::StepOut);
                match handle.recv_event() {
                    Ok(DebugEvent::Paused {
                        reason,
                        file,
                        line,
                        fn_name,
                    }) => {
                        current_file = file;
                        current_line = line;
                        print_pause_banner(
                            &reason,
                            &current_file,
                            current_line,
                            &fn_name,
                            &sources,
                        );
                    }
                    Ok(DebugEvent::Terminated { exit_code }) => {
                        println!("\n[Process finished with exit code: {exit_code}]");
                        is_running = false;
                    }
                    _ => break,
                }
            }
            "l" | "list" => {
                let count: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
                print_source_context(&current_file, current_line, count, &sources);
            }
            "bt" | "backtrace" => {
                let (reply_tx, reply_rx) = mpsc::channel();
                let _ = handle.send(DebugCommand::GetStackTrace(reply_tx));
                if let Ok(frames) = reply_rx.recv() {
                    println!("Call stack (most recent frame first):");
                    for f in frames {
                        println!("  #{} {}() at {}:{}", f.id, f.fn_name, f.file, f.line);
                    }
                }
            }
            "locals" => {
                let (reply_tx, reply_rx) = mpsc::channel();
                let _ = handle.send(DebugCommand::GetVariables {
                    frame_id: 0,
                    reply: reply_tx,
                });
                if let Ok(vars) = reply_rx.recv() {
                    println!("Local variables in scope:");
                    let mut sorted: Vec<(String, Value)> = vars.into_iter().collect();
                    sorted.sort_by(|a, b| a.0.cmp(&b.0));
                    for (k, v) in sorted {
                        println!("  {k} = {v}");
                    }
                }
            }
            "p" | "print" => {
                if parts.len() < 2 {
                    println!("Usage: print <expression>");
                    continue;
                }
                let expr = trimmed[parts[0].len()..].trim();
                let (reply_tx, reply_rx) = mpsc::channel();
                let _ = handle.send(DebugCommand::Evaluate {
                    expr: expr.to_string(),
                    frame_id: 0,
                    reply: reply_tx,
                });
                match reply_rx.recv() {
                    Ok(Ok(val)) => println!("$ = {val}"),
                    Ok(Err(err)) => println!("Evaluation error: {err}"),
                    Err(_) => println!("Error receiving evaluation result"),
                }
            }
            "q" | "quit" => {
                let _ = handle.send(DebugCommand::Quit);
                break;
            }
            other => {
                println!("Unknown command: '{other}'. Type 'help' for options.");
            }
        }
    }

    let _ = worker.join();
    Ok(())
}

fn print_help() {
    println!("\nSyntra Debugger Commands:");
    println!("  b, break <line> | <file:line>  Set a breakpoint at line number");
    println!("  d, delete <line>                Delete a breakpoint");
    println!("  c, continue                     Continue execution until next breakpoint");
    println!("  s, step                         Step into next statement/function");
    println!("  n, next                         Step over next statement");
    println!("  fin, finish                     Step out of current function");
    println!("  l, list [n]                     List source lines around current instruction");
    println!("  bt, backtrace                   Print the call stack frames");
    println!("  locals                          Print all variables in current local scope");
    println!("  p, print <expr>                 Evaluate expression in current scope");
    println!("  h, help                         Display this help menu");
    println!("  q, quit                         Exit the debugger session\n");
}

fn print_pause_banner(
    reason: &PauseReason,
    file: &str,
    line: usize,
    fn_name: &str,
    sources: &HashMap<String, String>,
) {
    let reason_str = match reason {
        PauseReason::Entry => "Paused at entry".to_string(),
        PauseReason::Breakpoint { .. } => "Breakpoint hit".to_string(),
        PauseReason::Step => "Stepped".to_string(),
        PauseReason::Exception(err) => format!("Exception caught: {err}"),
    };

    println!("\n[*] {reason_str} in {fn_name}() ({file}:{line})");
    print_source_context(file, line, 2, sources);
}

fn print_source_context(
    file: &str,
    current_line: usize,
    radius: usize,
    sources: &HashMap<String, String>,
) {
    let base_name = Path::new(file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file);

    let content = sources.get(file).or_else(|| sources.get(base_name));
    if let Some(src) = content {
        let lines: Vec<&str> = src.lines().collect();
        let start = if current_line > radius {
            current_line - radius
        } else {
            1
        };
        let end = (current_line + radius).min(lines.len());

        for l in start..=end {
            let marker = if l == current_line { "->" } else { "  " };
            let line_text = lines.get(l - 1).unwrap_or(&"");
            println!("  {marker} {l:4} | {line_text}");
        }
    }
}

/// Runs the official Debug Adapter Protocol (DAP) JSON-RPC server on stdin/stdout
pub fn run_dap_server() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let mut debug_handle: Option<DebugHandle> = None;
    let mut worker_thread: Option<thread::JoinHandle<Result<Value, syntra_runtime::RuntimeError>>> =
        None;

    let mut sources_cache: HashMap<String, String> = HashMap::new();
    let mut seq = 1;

    loop {
        // Read Content-Length header
        let mut header = String::new();
        if reader.read_line(&mut header).unwrap_or(0) == 0 {
            break;
        }

        if !header.starts_with("Content-Length:") {
            continue;
        }

        let len: usize = header["Content-Length:".len()..]
            .trim()
            .parse()
            .unwrap_or(0);

        // Read separator \r\n
        let mut blank = String::new();
        let _ = reader.read_line(&mut blank);

        let mut body_bytes = vec![0u8; len];
        if reader.read_exact(&mut body_bytes).is_err() {
            break;
        }

        let body_str = String::from_utf8_lossy(&body_bytes);
        let req = match JsonParser::parse(&body_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let req_seq = req.get("seq").and_then(|v| v.as_i64()).unwrap_or(0);
        let command = req.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let args = req.get("arguments").cloned().unwrap_or(JsonValue::Null);

        match command {
            "initialize" => {
                let mut caps = HashMap::new();
                caps.insert(
                    "supportsConfigurationDoneRequest".to_string(),
                    JsonValue::Bool(true),
                );
                caps.insert(
                    "supportsFunctionBreakpoints".to_string(),
                    JsonValue::Bool(true),
                );
                caps.insert(
                    "supportsEvaluateForHovers".to_string(),
                    JsonValue::Bool(true),
                );
                caps.insert("supportsStepBack".to_string(), JsonValue::Bool(false));

                send_dap_response(
                    &mut writer,
                    &mut seq,
                    req_seq,
                    command,
                    true,
                    Some(JsonValue::Object(caps)),
                );

                // Send initialized event
                send_dap_event(&mut writer, &mut seq, "initialized", None);
            }
            "launch" => {
                let program_path = args.get("program").and_then(|v| v.as_str()).unwrap_or("");
                let path = Path::new(program_path);

                if let Err(mismatches) = crate::package::verify_package_consumption(path) {
                    eprintln!(
                        "❌ Security Error: Dependency verification failed during DAP launch:"
                    );
                    for m in &mismatches {
                        eprintln!("   • {m}");
                    }
                    send_dap_response(&mut writer, &mut seq, req_seq, command, false, None);
                    continue;
                }

                let mut cmd_args = Vec::new();
                if let Some(arr) = args.get("args").and_then(|v| v.as_array()) {
                    for a in arr {
                        if let Some(s) = a.as_str() {
                            cmd_args.push(s.to_string());
                        }
                    }
                }

                let mut loader = syntra_frontend::loader::ModuleLoader::new();
                if let Ok(prog) = loader.load_program(path) {
                    for (p, src) in &loader.sources {
                        sources_cache.insert(p.display().to_string(), src.clone());
                    }
                    if let Ok(src) = std::fs::read_to_string(program_path) {
                        sources_cache.insert(program_path.to_string(), src);
                    }

                    let (debug_state, handle) = create_debug_channel();
                    let sources_w = sources_cache.clone();
                    let file_w = program_path.to_string();

                    worker_thread = Some(thread::spawn(move || {
                        let mut interp = Interpreter::new();
                        interp.set_args(cmd_args);
                        interp.attach_debugger(debug_state);
                        for (f, s) in sources_w {
                            interp.set_source(&f, &s);
                        }
                        interp.current_file = file_w;
                        interp.run_program(&prog)
                    }));

                    let stop_on_entry = args
                        .get("stopOnEntry")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    // Wait for worker to enter execution and hold at entry
                    if let Ok(DebugEvent::Paused { .. }) = handle.recv_event() {
                        if stop_on_entry {
                            let mut body = HashMap::new();
                            body.insert(
                                "reason".to_string(),
                                JsonValue::String("entry".to_string()),
                            );
                            body.insert("threadId".to_string(), JsonValue::Number(1.0));
                            body.insert("allThreadsStopped".to_string(), JsonValue::Bool(true));
                            send_dap_event(
                                &mut writer,
                                &mut seq,
                                "stopped",
                                Some(JsonValue::Object(body)),
                            );
                        }
                    }

                    debug_handle = Some(handle);
                    send_dap_response(&mut writer, &mut seq, req_seq, command, true, None);
                } else {
                    send_dap_response(&mut writer, &mut seq, req_seq, command, false, None);
                }
            }
            "setBreakpoints" => {
                let source_path = args
                    .get("source")
                    .and_then(|s| s.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("");
                let mut verified_bps = Vec::new();

                if let Some(bps) = args.get("breakpoints").and_then(|b| b.as_array()) {
                    for bp in bps {
                        let line = bp.get("line").and_then(|l| l.as_i64()).unwrap_or(0) as usize;
                        if let Some(ref handle) = debug_handle {
                            let _ = handle.send(DebugCommand::SetBreakpoint {
                                file: source_path.to_string(),
                                line,
                            });
                        }
                        let mut bp_obj = HashMap::new();
                        bp_obj.insert("verified".to_string(), JsonValue::Bool(true));
                        bp_obj.insert("line".to_string(), JsonValue::Number(line as f64));
                        verified_bps.push(JsonValue::Object(bp_obj));
                    }
                }

                let mut body = HashMap::new();
                body.insert("breakpoints".to_string(), JsonValue::Array(verified_bps));
                send_dap_response(
                    &mut writer,
                    &mut seq,
                    req_seq,
                    command,
                    true,
                    Some(JsonValue::Object(body)),
                );
            }
            "configurationDone" => {
                send_dap_response(&mut writer, &mut seq, req_seq, command, true, None);
                // Continue to first breakpoint or pause
                if let Some(ref handle) = debug_handle {
                    let _ = handle.send(DebugCommand::Continue);
                    check_dap_event(handle, &mut writer, &mut seq);
                }
            }
            "threads" => {
                let mut thread_obj = HashMap::new();
                thread_obj.insert("id".to_string(), JsonValue::Number(1.0));
                thread_obj.insert("name".to_string(), JsonValue::String("main".to_string()));

                let mut body = HashMap::new();
                body.insert(
                    "threads".to_string(),
                    JsonValue::Array(vec![JsonValue::Object(thread_obj)]),
                );
                send_dap_response(
                    &mut writer,
                    &mut seq,
                    req_seq,
                    command,
                    true,
                    Some(JsonValue::Object(body)),
                );
            }
            "stackTrace" => {
                let mut frames_arr = Vec::new();
                if let Some(ref handle) = debug_handle {
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = handle.send(DebugCommand::GetStackTrace(reply_tx));
                    if let Ok(frames) = reply_rx.recv() {
                        for f in frames {
                            let mut frame = HashMap::new();
                            frame.insert("id".to_string(), JsonValue::Number(f.id as f64));
                            frame.insert("name".to_string(), JsonValue::String(f.fn_name));
                            frame.insert("line".to_string(), JsonValue::Number(f.line as f64));
                            frame.insert("column".to_string(), JsonValue::Number(1.0));

                            let mut src_obj = HashMap::new();
                            src_obj.insert("path".to_string(), JsonValue::String(f.file));
                            frame.insert("source".to_string(), JsonValue::Object(src_obj));

                            frames_arr.push(JsonValue::Object(frame));
                        }
                    }
                }

                let mut body = HashMap::new();
                body.insert("stackFrames".to_string(), JsonValue::Array(frames_arr));
                send_dap_response(
                    &mut writer,
                    &mut seq,
                    req_seq,
                    command,
                    true,
                    Some(JsonValue::Object(body)),
                );
            }
            "scopes" => {
                let mut scope = HashMap::new();
                scope.insert("name".to_string(), JsonValue::String("Locals".to_string()));
                scope.insert("variablesReference".to_string(), JsonValue::Number(1000.0));
                scope.insert("expensive".to_string(), JsonValue::Bool(false));

                let mut body = HashMap::new();
                body.insert(
                    "scopes".to_string(),
                    JsonValue::Array(vec![JsonValue::Object(scope)]),
                );
                send_dap_response(
                    &mut writer,
                    &mut seq,
                    req_seq,
                    command,
                    true,
                    Some(JsonValue::Object(body)),
                );
            }
            "variables" => {
                let mut vars_arr = Vec::new();
                if let Some(ref handle) = debug_handle {
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = handle.send(DebugCommand::GetVariables {
                        frame_id: 0,
                        reply: reply_tx,
                    });
                    if let Ok(vars) = reply_rx.recv() {
                        let mut sorted: Vec<(String, Value)> = vars.into_iter().collect();
                        sorted.sort_by(|a, b| a.0.cmp(&b.0));
                        for (k, v) in sorted {
                            let mut var_obj = HashMap::new();
                            var_obj.insert("name".to_string(), JsonValue::String(k));
                            var_obj.insert("value".to_string(), JsonValue::String(v.to_string()));
                            var_obj
                                .insert("variablesReference".to_string(), JsonValue::Number(0.0));
                            vars_arr.push(JsonValue::Object(var_obj));
                        }
                    }
                }

                let mut body = HashMap::new();
                body.insert("variables".to_string(), JsonValue::Array(vars_arr));
                send_dap_response(
                    &mut writer,
                    &mut seq,
                    req_seq,
                    command,
                    true,
                    Some(JsonValue::Object(body)),
                );
            }
            "continue" => {
                if let Some(ref handle) = debug_handle {
                    let _ = handle.send(DebugCommand::Continue);
                    let mut body = HashMap::new();
                    body.insert("allThreadsContinued".to_string(), JsonValue::Bool(true));
                    send_dap_response(
                        &mut writer,
                        &mut seq,
                        req_seq,
                        command,
                        true,
                        Some(JsonValue::Object(body)),
                    );
                    check_dap_event(handle, &mut writer, &mut seq);
                }
            }
            "next" => {
                if let Some(ref handle) = debug_handle {
                    let _ = handle.send(DebugCommand::StepOver);
                    send_dap_response(&mut writer, &mut seq, req_seq, command, true, None);
                    check_dap_event(handle, &mut writer, &mut seq);
                }
            }
            "stepIn" => {
                if let Some(ref handle) = debug_handle {
                    let _ = handle.send(DebugCommand::StepInto);
                    send_dap_response(&mut writer, &mut seq, req_seq, command, true, None);
                    check_dap_event(handle, &mut writer, &mut seq);
                }
            }
            "stepOut" => {
                if let Some(ref handle) = debug_handle {
                    let _ = handle.send(DebugCommand::StepOut);
                    send_dap_response(&mut writer, &mut seq, req_seq, command, true, None);
                    check_dap_event(handle, &mut writer, &mut seq);
                }
            }
            "evaluate" => {
                let expr = args
                    .get("expression")
                    .and_then(|e| e.as_str())
                    .unwrap_or("");
                let mut res_str = String::new();
                if let Some(ref handle) = debug_handle {
                    let (reply_tx, reply_rx) = mpsc::channel();
                    let _ = handle.send(DebugCommand::Evaluate {
                        expr: expr.to_string(),
                        frame_id: 0,
                        reply: reply_tx,
                    });
                    if let Ok(res) = reply_rx.recv() {
                        match res {
                            Ok(v) => res_str = v.to_string(),
                            Err(err) => res_str = format!("Error: {err}"),
                        }
                    }
                }

                let mut body = HashMap::new();
                body.insert("result".to_string(), JsonValue::String(res_str));
                body.insert("variablesReference".to_string(), JsonValue::Number(0.0));
                send_dap_response(
                    &mut writer,
                    &mut seq,
                    req_seq,
                    command,
                    true,
                    Some(JsonValue::Object(body)),
                );
            }
            "disconnect" => {
                if let Some(ref handle) = debug_handle {
                    let _ = handle.send(DebugCommand::Quit);
                }
                send_dap_response(&mut writer, &mut seq, req_seq, command, true, None);
                break;
            }
            _ => {
                send_dap_response(&mut writer, &mut seq, req_seq, command, true, None);
            }
        }
    }

    if let Some(w) = worker_thread {
        let _ = w.join();
    }
}

fn check_dap_event(handle: &DebugHandle, writer: &mut io::StdoutLock, seq: &mut i64) {
    if let Ok(ev) = handle.recv_event() {
        match ev {
            DebugEvent::Paused { reason, .. } => {
                let reason_str = match reason {
                    PauseReason::Breakpoint { .. } => "breakpoint",
                    PauseReason::Step => "step",
                    PauseReason::Entry => "entry",
                    PauseReason::Exception(_) => "exception",
                };
                let mut body = HashMap::new();
                body.insert(
                    "reason".to_string(),
                    JsonValue::String(reason_str.to_string()),
                );
                body.insert("threadId".to_string(), JsonValue::Number(1.0));
                body.insert("allThreadsStopped".to_string(), JsonValue::Bool(true));
                send_dap_event(writer, seq, "stopped", Some(JsonValue::Object(body)));
            }
            DebugEvent::Terminated { .. } => {
                send_dap_event(writer, seq, "terminated", None);
            }
            _ => {}
        }
    }
}

fn send_dap_response(
    writer: &mut io::StdoutLock,
    seq: &mut i64,
    request_seq: i64,
    command: &str,
    success: bool,
    body: Option<JsonValue>,
) {
    let mut map = HashMap::new();
    map.insert("seq".to_string(), JsonValue::Number(*seq as f64));
    map.insert(
        "type".to_string(),
        JsonValue::String("response".to_string()),
    );
    map.insert(
        "request_seq".to_string(),
        JsonValue::Number(request_seq as f64),
    );
    map.insert(
        "command".to_string(),
        JsonValue::String(command.to_string()),
    );
    map.insert("success".to_string(), JsonValue::Bool(success));
    if let Some(b) = body {
        map.insert("body".to_string(), b);
    }
    *seq += 1;

    let json_text = JsonValue::Object(map).to_json_string();
    let msg = format!("Content-Length: {}\r\n\r\n{}", json_text.len(), json_text);
    let _ = writer.write_all(msg.as_bytes());
    let _ = writer.flush();
}

fn send_dap_event(
    writer: &mut io::StdoutLock,
    seq: &mut i64,
    event: &str,
    body: Option<JsonValue>,
) {
    let mut map = HashMap::new();
    map.insert("seq".to_string(), JsonValue::Number(*seq as f64));
    map.insert("type".to_string(), JsonValue::String("event".to_string()));
    map.insert("event".to_string(), JsonValue::String(event.to_string()));
    if let Some(b) = body {
        map.insert("body".to_string(), b);
    }
    *seq += 1;

    let json_text = JsonValue::Object(map).to_json_string();
    let msg = format!("Content-Length: {}\r\n\r\n{}", json_text.len(), json_text);
    let _ = writer.write_all(msg.as_bytes());
    let _ = writer.flush();
}
