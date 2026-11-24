use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::exit;

#[derive(ClapParser)]
#[command(name = "aether")]
#[command(about = "The Aether Programming Language & Fast Execution Platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Direct file execution shortcut: 'ae file.ae' or 'aether file.ae'
    file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute an Aether source file
    Run {
        /// Path to the .ae source file
        file: PathBuf,
    },
    /// Check an Aether source file for syntax and type errors
    Check {
        /// Path to the .ae source file
        file: PathBuf,
        /// Print the parsed Abstract Syntax Tree
        #[arg(long)]
        dump_ast: bool,
        /// Lower to AIR and print the intermediate representation
        #[arg(long)]
        dump_air: bool,
    },
    /// Lower to AIR (Aether Intermediate Representation) and verify
    Air {
        /// Path to the .ae source file
        file: PathBuf,
    },
}

fn execute_file(file: &PathBuf) {
    let filename = file.display().to_string();
    let source = match fs::read_to_string(file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{}': {}", filename, err);
            exit(1);
        }
    };

    // Stage 1: Syntax Analysis
    let program = match aether_syntax::parse(&source) {
        Ok(prog) => prog,
        Err(diagnostics) => {
            eprintln!("❌ Syntax Error:\n");
            for diag in diagnostics {
                diag.emit(&filename, &source);
            }
            exit(1);
        }
    };

    // Stage 2: Type Checking
    if let Err(diagnostics) = aether_frontend::check(&program) {
        eprintln!("❌ Type Error:\n");
        for diag in diagnostics {
            diag.emit(&filename, &source);
        }
        exit(1);
    }

    // Stage 3: Fast Execution
    match aether_runtime::run(&program) {
        Ok(result) => {
            match result {
                aether_runtime::Value::Void => {}
                aether_runtime::Value::Int(code) => {
                    println!("\n[Program exited with code: {}]", code);
                }
                other => {
                    println!("\n[Program returned: {}]", other);
                }
            }
        }
        Err(err) => {
            eprintln!("\n❌ Runtime Error: {}", err);
            exit(1);
        }
    }
}

fn check_file(file: &PathBuf, dump_ast: bool, dump_air: bool) {
    let filename = file.display().to_string();
    let source = match fs::read_to_string(file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{}': {}", filename, err);
            exit(1);
        }
    };

    println!("🔍 Checking '{}'...", filename);

    let program = match aether_syntax::parse(&source) {
        Ok(prog) => {
            println!("✅ Syntax check passed! Successfully parsed {} top-level item(s).", prog.items.len());
            if dump_ast {
                println!("\n--- Abstract Syntax Tree (AST) ---");
                println!("{:#?}", prog);
            }
            prog
        }
        Err(diagnostics) => {
            eprintln!("❌ Syntax check failed with {} error(s):\n", diagnostics.len());
            for diag in diagnostics {
                diag.emit(&filename, &source);
            }
            exit(1);
        }
    };

    match aether_frontend::check(&program) {
        Ok(()) => {
            println!("✅ Type check passed! 0 semantic errors found.");
        }
        Err(diagnostics) => {
            eprintln!("❌ Type check failed with {} error(s):\n", diagnostics.len());
            for diag in diagnostics {
                diag.emit(&filename, &source);
            }
            exit(1);
        }
    }

    if dump_air {
        emit_air(&program);
    }
}

fn emit_air(program: &aether_syntax::ast::Program) {
    match aether_air::lower_and_verify(program) {
        Ok(module) => {
            println!("✅ AIR Invariant Verification passed! (AIR-001, AIR-002, AIR-003 enforced)");
            println!("\n--- Aether Intermediate Representation (AIR) ---");
            println!("{}", aether_air::print_air_module(&module));
        }
        Err(diagnostics) => {
            eprintln!("❌ AIR Verification failed with {} error(s):\n", diagnostics.len());
            for diag in diagnostics {
                eprintln!("[{}] {}", diag.code, diag.message);
            }
            exit(1);
        }
    }
}

pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file }) => {
            execute_file(&file);
        }
        Some(Commands::Check { file, dump_ast, dump_air }) => {
            check_file(&file, dump_ast, dump_air);
        }
        Some(Commands::Air { file }) => {
            let filename = file.display().to_string();
            let source = match fs::read_to_string(&file) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Error reading file '{}': {}", filename, err);
                    exit(1);
                }
            };
            if let Ok(program) = aether_syntax::parse(&source) {
                emit_air(&program);
            }
        }
        None => {
            if let Some(file) = cli.file {
                execute_file(&file);
            } else {
                eprintln!("Usage: ae <file.ae>  OR  aether run <file.ae>  OR  aether air <file.ae>");
                exit(1);
            }
        }
    }
}
