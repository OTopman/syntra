//! # Syntra Unified Developer CLI
//!
//! Provides the primary command-line tool (`syn` / `syntra`) for running, building, testing,
//! formatting, linting, packaging, and profiling Syntra code, as well as launching the Language Server (LSP).

use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

pub mod debug;
pub mod doc;
pub mod fmt;
pub mod learn;
pub mod lsp;
pub mod mail;
pub mod package;

use learn::{handle_learn_command, LearnCommand};
use mail::{handle_mail_command, MailCommand};

#[derive(ClapParser)]
#[command(name = "syn")]
#[command(about = "The Syntra Programming Language & Whole-Program Adaptive Platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Direct file execution shortcut: 'syn file.syn' or 'syntra file.syn'
    file: Option<PathBuf>,

    /// Arguments passed to the Syntra program
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Fast-execute a Syntra source file
    Run {
        /// Path to the .syn source file
        file: PathBuf,
        /// Enable Self-Adapting Runtime (SAR) dynamic governor
        #[arg(long)]
        sar: bool,
        /// Arguments passed to the Syntra program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Compile a Syntra source file into a native standalone binary
    Build {
        /// Path to the .syn source file
        file: PathBuf,
        /// Name of the output executable binary
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Target triple for cross-compilation (e.g. x86_64-apple-darwin, aarch64-unknown-linux-gnu)
        #[arg(long)]
        target: Option<String>,
        /// Enable Profile-Guided Optimization (PGO) using profile.synlock
        #[arg(long)]
        pgo: bool,
        /// Path to custom profile snapshot or lockfile for PGO
        #[arg(long)]
        profile: Option<PathBuf>,
    },
    /// Check a Syntra source file for syntax and type errors
    Check {
        /// Path to the .syn source file
        file: PathBuf,
        /// Print the parsed Abstract Syntax Tree
        #[arg(long)]
        dump_ast: bool,
        /// Lower to AIR/SIR and print the intermediate representation
        #[arg(long, visible_alias = "dump-sir")]
        dump_air: bool,
    },
    /// Lower to AIR / SIR (Syntra Intermediate Representation) and verify
    #[command(alias = "sir")]
    Air {
        /// Path to the .syn source file
        file: PathBuf,
    },
    /// Optimize a Syntra source file (Constant Folding, DCE) and dump AIR
    Opt {
        /// Path to the .syn source file
        file: PathBuf,
    },
    /// Consult the Syntra Intelligence Engine (AIE) for strategy recommendations
    Plan {
        /// Path to the .syn source file
        file: PathBuf,
    },
    /// Inspect cryptographic BuildIdentity manifest for reproducible builds
    Manifest {
        /// Path to the .syn source file
        file: PathBuf,
    },
    /// Generate API documentation from AST doc-comments (///)
    Doc {
        /// Path to the .syn source file or directory
        file: PathBuf,
        /// Export format (markdown or html)
        #[arg(short, long, default_value = "markdown")]
        format: String,
        /// Output file or directory path (defaults to stdout or docs_site/)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Initialize a new Syntra package in the current directory
    Init {
        /// Package name (defaults to current directory name)
        name: Option<String>,
    },
    /// Create a new Syntra project in a new directory
    New {
        /// Path/name of the new project directory
        path: PathBuf,
    },
    /// Add a dependency to Package.syn / syntra.toml
    Add {
        /// Dependency package name (optional if --git is specified)
        package: Option<String>,
        /// Path to local dependency directory
        #[arg(long)]
        path: Option<String>,
        /// Version requirement
        #[arg(long)]
        version: Option<String>,
        /// Remote Git repository URL (e.g. https://github.com/org/repo.git)
        #[arg(long)]
        git: Option<String>,
        /// Git branch to checkout
        #[arg(long)]
        branch: Option<String>,
        /// Git tag to checkout
        #[arg(long)]
        tag: Option<String>,
        /// Git commit revision to checkout
        #[arg(long)]
        rev: Option<String>,
    },
    /// Remove a dependency from syntra.toml
    Remove {
        /// Dependency package name to remove
        package: String,
    },
    /// Install dependencies specified in syntra.toml and generate syntra.lock
    Install,
    /// Freeze dependencies and verify cryptographic checksums in Package.lock
    Lock {
        /// Check if the existing lockfile is up to date without updating it
        #[arg(long)]
        check: bool,
    },
    /// Package the project into a distributable archive (.synpkg)
    Pack {
        /// Destination archive path
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Bundle all resolved dependencies for offline compilation
        #[arg(long)]
        vendor: bool,
    },
    /// Format Syntra source files with canonical styling
    Fmt {
        /// Path to specific .syn file or directory to format (defaults to current project)
        file: Option<PathBuf>,
        /// Check if files are formatted without writing changes
        #[arg(long)]
        check: bool,
    },
    /// Launch the Syntra Language Server (LSP) for real-time editor integration
    Lsp,
    /// Perform whole-program escape analysis and call-graph verification
    Analyze {
        /// Path to the .syn source file
        file: PathBuf,
    },
    /// Run autonomous compilation experimentation, sandbox differential verification, and Pareto ranking
    Experiment {
        /// Path to the .syn source file
        file: PathBuf,
        /// Number of benchmark sample iterations per verified candidate
        #[arg(short, long, default_value = "5")]
        iterations: usize,
    },
    /// Commit an approved optimization candidate strategy for production builds
    Approve {
        /// Candidate identifier (e.g. cand_1_...)
        candidate_id: String,
        /// Target package or project directory (defaults to current dir)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Profile runtime execution telemetry, function call heatmaps, and memory allocation rates
    Profile {
        /// Path to the .syn source file
        file: PathBuf,
        /// Export profile snapshot JSON path (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Write canonical, deterministic profile lockfile (profile.synlock)
        #[arg(long)]
        lock: bool,
        /// Arguments passed to the Syntra program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run autonomous profile-guided optimization tuning loop (Profile -> Sandbox -> Learn -> Build)
    Tune {
        /// Path to the .syn source file
        file: PathBuf,
        /// Number of benchmark experiment iterations
        #[arg(short, long, default_value = "5")]
        iterations: usize,
        /// Name of the output executable binary
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Arguments passed to program during profiling run
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Debug a Syntra program with interactive terminal debugger or DAP server
    Debug {
        /// Path to the .syn source file (optional if --dap is specified)
        file: Option<PathBuf>,
        /// Run in Debug Adapter Protocol (DAP) server mode over stdio
        #[arg(long)]
        dap: bool,
        /// Arguments passed to the Syntra program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Manage Syntra Learning System (SLS) models, training pipelines, and governance
    Learn {
        #[command(subcommand)]
        command: LearnCommand,
    },
    /// Inspect RFC 5322 MIME messages and verify SMTP relays
    Mail {
        #[command(subcommand)]
        command: MailCommand,
    },
}

fn load_program_or_exit(file: &PathBuf) -> (syntra_syntax::ast::Program, String) {
    let filename = file.display().to_string();

    // Verify package integrity at compiler consumption time before loading AST
    if let Err(mismatches) = package::verify_package_consumption(file) {
        eprintln!(
            "❌ Security Error: Dependency verification failed at compiler consumption time:\n"
        );
        for m in mismatches {
            eprintln!("   • {m}");
        }
        eprintln!("\nRefusing to compile or execute tampered dependencies. Run 'syn install' or investigate tampering.");
        exit(1);
    }

    let mut loader = syntra_frontend::ModuleLoader::new();
    match loader.load_program(file) {
        Ok(prog) => {
            let canonical = file.canonicalize().unwrap_or_else(|_| file.clone());
            let source = loader.sources.get(&canonical).cloned().unwrap_or_default();
            (prog, source)
        }
        Err(diagnostics) => {
            eprintln!("❌ Error loading '{filename}':\n");
            let fallback_source = fs::read_to_string(file).unwrap_or_default();
            for diag in diagnostics {
                diag.emit(&filename, &fallback_source);
            }
            exit(1);
        }
    }
}

fn execute_file(file: &PathBuf, args: Vec<String>, sar: bool) {
    let filename = file.display().to_string();
    let (program, source) = load_program_or_exit(file);

    // Stage 2: Type Checking
    if let Err(diagnostics) = syntra_frontend::check(&program) {
        eprintln!("❌ Type Error:\n");
        for diag in diagnostics {
            diag.emit(&filename, &source);
        }
        exit(1);
    }

    let mut sar_engine = if sar {
        println!("⚡ [SAR] Initializing Self-Adapting Runtime Governor (Specification 06)...");
        let mut engine = syntra_runtime::SarEngine::new_active();
        engine.record_task_queue(1, 0);
        Some(engine)
    } else {
        None
    };

    // Stage 3: Fast Execution
    let result = syntra_runtime::run_with_args(&program, args);

    if let Some(ref mut engine) = sar_engine {
        engine.record_allocation(1024 * 64);
        let action = engine.evaluate();
        println!("⚡ [SAR] Evaluated runtime adaptation: {action:?}");
        println!("\n{}", engine.report_summary());
    }

    match result {
        Ok(res) => match res {
            syntra_runtime::Value::Void => {}
            syntra_runtime::Value::Int(code) => {
                println!("\n[Program exited with code: {code}]");
            }
            other => {
                println!("\n[Program returned: {other}]");
            }
        },
        Err(err) => {
            eprintln!("\n❌ Runtime Error: {err}");
            exit(1);
        }
    }
}

fn build_native_binary(
    file: &PathBuf,
    output: Option<PathBuf>,
    target: Option<String>,
    pgo: bool,
    profile_override: Option<PathBuf>,
) {
    let filename = file.display().to_string();
    let target_name = output.unwrap_or_else(|| {
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        PathBuf::from(stem)
    });

    let host_triple = if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
        "aarch64-apple-darwin"
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "macos") {
        "x86_64-apple-darwin"
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "linux") {
        "aarch64-unknown-linux-gnu"
    } else {
        "x86_64-apple-darwin"
    };

    let effective_target = target.as_deref().unwrap_or(host_triple);

    println!("🔨 Building native binary for '{filename}' (target: {effective_target})...");

    let (program, source) = load_program_or_exit(file);

    // 2. Semantic Analysis
    if let Err(diagnostics) = syntra_frontend::check(&program) {
        eprintln!("❌ Type Error:\n");
        for diag in diagnostics {
            diag.emit(&filename, &source);
        }
        exit(1);
    }

    // 3. AIR Lowering & Invariant Verification
    let mut air_module = match syntra_air::lower_and_verify(&program) {
        Ok(m) => m,
        Err(diagnostics) => {
            eprintln!("❌ AIR Verification Error:\n");
            for diag in diagnostics {
                eprintln!("[{}] {}", diag.code, diag.message);
            }
            exit(1);
        }
    };

    // Check PackageManifest for optimization settings
    let manifest_opt = syntra_frontend::find_package_manifest(file)
        .and_then(|p| fs::read_to_string(&p).ok())
        .map(|c| syntra_frontend::PackageManifest::parse(&c));

    let should_pgo = pgo
        || profile_override.is_some()
        || manifest_opt
            .as_ref()
            .and_then(|m| m.optimization.pgo.as_deref())
            == Some("locked");

    // Check for developer-approved SECE strategy (.syntra/approved_strategy.json)
    let approved_strategy_opt = syntra_sandbox::ApprovedStrategy::load_from_dir(Path::new("."))
        .or_else(|| {
            file.parent()
                .and_then(syntra_sandbox::ApprovedStrategy::load_from_dir)
        });

    let mut applied_strategies = Vec::new();

    // 4. Optimization Pipeline (Developer Approved Strategy, PGO, or Default Pipeline)
    let pipeline = if let Some(approved) = approved_strategy_opt {
        println!("🎯 Approved Strategy: Consuming '.syntra/approved_strategy.json' (Candidate: '{}', Policy: '{}')", approved.approved_candidate_id, approved.policy);
        match approved.to_strategy_kind() {
            syntra_sandbox::candidate::StrategyKind::Baseline => {
                applied_strategies.push(format!("Approved({})", approved.approved_candidate_id));
                syntra_opt::OptimizationPipeline::empty()
            }
            syntra_sandbox::candidate::StrategyKind::EscapePromoted => {
                applied_strategies.push(format!("Approved({})", approved.approved_candidate_id));
                applied_strategies.push("EscapePromotion".to_string());
                syntra_opt::OptimizationPipeline::escape_promotion_only()
            }
            syntra_sandbox::candidate::StrategyKind::ConstantFolded => {
                applied_strategies.push(format!("Approved({})", approved.approved_candidate_id));
                applied_strategies.push("ConstantFolding".to_string());
                applied_strategies.push("DeadCodeElimination".to_string());
                syntra_opt::OptimizationPipeline::constant_folding_dce()
            }
            syntra_sandbox::candidate::StrategyKind::AggressiveCombined => {
                applied_strategies.push(format!("Approved({})", approved.approved_candidate_id));
                applied_strategies.push("EscapePromotion".to_string());
                applied_strategies.push("ConstantFolding".to_string());
                applied_strategies.push("DeadCodeElimination".to_string());
                syntra_opt::OptimizationPipeline::aggressive_combined()
            }
        }
    } else {
        applied_strategies.extend(vec![
            "ConstantFolding".to_string(),
            "DeadCodeElimination".to_string(),
            "EscapePromotion".to_string(),
        ]);
        if should_pgo {
            let mut resolved_profile = None;
            if let Some(ref custom_p) = profile_override {
                if custom_p.is_file() {
                    resolved_profile = Some(custom_p.clone());
                }
            }
            if resolved_profile.is_none() {
                let synlock = Path::new("profile.synlock");
                if synlock.is_file() {
                    resolved_profile = Some(synlock.to_path_buf());
                } else if let Some(parent) = file.parent() {
                    let parent_synlock = parent.join("profile.synlock");
                    if parent_synlock.is_file() {
                        resolved_profile = Some(parent_synlock);
                    }
                }
            }
            if resolved_profile.is_none() {
                let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
                let json_cand = Path::new(".syntra")
                    .join("profiles")
                    .join(format!("{stem}.profile.json"));
                if json_cand.is_file() {
                    resolved_profile = Some(json_cand);
                }
            }

            if let Some(prof_path) = resolved_profile {
                if let Ok(content) = fs::read_to_string(&prof_path) {
                    if let Some(lockfile) =
                        syntra_runtime::RuntimeProfileData::from_synlock(&content)
                    {
                        println!(
                            "🎯 PGO: Consuming profile lockfile '{}' (Integrity: {})",
                            prof_path.display(),
                            lockfile.integrity_hash
                        );
                        let mut hot_set = std::collections::HashSet::new();
                        let mut call_counts = std::collections::HashMap::new();
                        for (k, v) in &lockfile.functions {
                            if v.is_hot {
                                hot_set.insert(k.clone());
                            }
                            call_counts.insert(k.clone(), v.call_count);
                        }
                        applied_strategies.push(format!("PGO({})", lockfile.integrity_hash));
                        syntra_opt::OptimizationPipeline::with_pgo(
                            syntra_opt::PgoOptimizationPass::new(hot_set, call_counts),
                        )
                    } else {
                        syntra_opt::OptimizationPipeline::default_pipeline()
                    }
                } else {
                    syntra_opt::OptimizationPipeline::default_pipeline()
                }
            } else {
                if pgo {
                    println!("ℹ️ PGO: No profile.synlock found. Build proceeding with standard static optimization.");
                }
                syntra_opt::OptimizationPipeline::default_pipeline()
            }
        } else {
            syntra_opt::OptimizationPipeline::default_pipeline()
        }
    };

    let passes_applied = pipeline.optimize(&mut air_module).unwrap_or(0);
    println!(
        "⚡ Optimization pipeline applied {passes_applied} pass(es) with verified invariants."
    );

    // 5. Cryptographic Build Identity (Specification 09)
    let manifest = syntra_provenance::BuildIdentity::compute(
        &source,
        &air_module,
        effective_target,
        &applied_strategies,
    );
    println!(
        "🔒 Build Identity Manifest: {}",
        manifest.composite_manifest_id
    );

    // 6. Cranelift Native Machine Code Emission
    let temp_obj = target_name.with_extension("o");
    if let Err(err) = syntra_backend_cranelift::compile_module_to_object_target(
        &air_module,
        &temp_obj,
        Some(effective_target),
    ) {
        eprintln!("❌ Native Compilation Error: {err}");
        exit(1);
    }

    // 7. Link with system linker
    if let Err(err) = syntra_backend_cranelift::link_executable_target(
        &temp_obj,
        &target_name,
        Some(effective_target),
    ) {
        eprintln!("❌ Linker Error: {err}");
        let _ = fs::remove_file(&temp_obj);
        exit(1);
    }

    // Clean up temporary .o
    let _ = fs::remove_file(&temp_obj);

    println!(
        "✅ Successfully compiled native executable: {}",
        target_name.display()
    );
    println!("🚀 Run it directly: ./{}", target_name.display());
}

fn optimize_file(file: &PathBuf) {
    let (program, _source) = load_program_or_exit(file);

    let mut air_module = match syntra_air::lower_and_verify(&program) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags {
                eprintln!("[{}] {}", d.code, d.message);
            }
            exit(1);
        }
    };

    println!("⚡ Running Syntra Canonical Optimization Pipeline...");
    let pipeline = syntra_opt::OptimizationPipeline::default_pipeline();
    match pipeline.optimize(&mut air_module) {
        Ok(passes) => {
            println!(
                "✅ Optimizations applied ({passes} passes executed). All AIR invariants verified!"
            );
            println!("\n--- Optimized AIR ---");
            println!("{}", syntra_air::print_air_module(&air_module));
        }
        Err(diags) => {
            eprintln!("❌ Optimization invariant check failed:");
            for d in diags {
                eprintln!("[{}] {}", d.code, d.message);
            }
            exit(1);
        }
    }
}

fn plan_file(file: &PathBuf) {
    let (program, _source) = load_program_or_exit(file);

    let air_module = match syntra_air::lower_and_verify(&program) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags {
                eprintln!("[{}] {}", d.code, d.message);
            }
            exit(1);
        }
    };

    let analysis = syntra_analysis::analyze_module(&air_module);

    let profile = if Path::new("profile.synlock").is_file() {
        fs::read_to_string("profile.synlock")
            .ok()
            .and_then(|c| syntra_runtime::RuntimeProfileData::from_synlock(&c))
            .map(|l| syntra_runtime::RuntimeProfileData::from_lockfile(&l))
    } else {
        None
    };

    let coordinator =
        syntra_intelligence::IntelligenceCoordinator::new(syntra_intelligence::PolicyMode::Approve);
    println!(
        "{}",
        coordinator.format_plan_report(&air_module, Some(&analysis), profile.as_ref())
    );
}

fn manifest_file(file: &PathBuf) {
    let (program, source) = load_program_or_exit(file);

    let air_module = match syntra_air::lower_and_verify(&program) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags {
                eprintln!("[{}] {}", d.code, d.message);
            }
            exit(1);
        }
    };

    let manifest = syntra_provenance::BuildIdentity::compute(
        &source,
        &air_module,
        "x86_64-apple-darwin",
        &[
            "ConstantFolding".to_string(),
            "DeadCodeElimination".to_string(),
        ],
    );
    println!("{}", manifest.to_manifest_text());
}

fn check_file(file: &PathBuf, dump_ast: bool, dump_air: bool) {
    let filename = file.display().to_string();
    println!("🔍 Checking '{filename}'...");

    let (program, source) = load_program_or_exit(file);
    println!(
        "✅ Syntax check passed! Successfully parsed {} top-level item(s).",
        program.items.len()
    );
    if dump_ast {
        println!("\n--- Abstract Syntax Tree (AST) ---");
        println!("{program:#?}");
    }

    match syntra_frontend::check(&program) {
        Ok(()) => {
            println!("✅ Type check passed! 0 semantic errors found.");
        }
        Err(diagnostics) => {
            eprintln!(
                "❌ Type check failed with {} error(s):\n",
                diagnostics.len()
            );
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

fn emit_air(program: &syntra_syntax::ast::Program) {
    match syntra_air::lower_and_verify(program) {
        Ok(module) => {
            println!("✅ AIR Invariant Verification passed! (AIR-001, AIR-002, AIR-003 enforced)");
            println!("\n--- Syntra Intermediate Representation (AIR) ---");
            println!("{}", syntra_air::print_air_module(&module));
        }
        Err(diagnostics) => {
            eprintln!(
                "❌ AIR Verification failed with {} error(s):\n",
                diagnostics.len()
            );
            for diag in diagnostics {
                eprintln!("[{}] {}", diag.code, diag.message);
            }
            exit(1);
        }
    }
}

fn generate_docs(file: &Path, format: &str, output: Option<PathBuf>) {
    doc::generate_docs(file, format, output);
}

fn start_repl() {
    println!("🔮 Syntra v0.1.0 Interactive REPL");
    println!("Type Syntra expressions or statements. Type 'exit' or Ctrl+D to exit.\n");

    let stdin = std::io::stdin();
    let mut interp = syntra_runtime::Interpreter::new();

    loop {
        use std::io::Write;
        print!("syn> ");
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => {
                println!("\nGoodbye!");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed == "exit" || trimmed == "quit" {
                    println!("Goodbye!");
                    break;
                }
                if trimmed.is_empty() {
                    continue;
                }
                match interp.eval_line(trimmed) {
                    Ok(Some(val)) => {
                        if !matches!(val, syntra_runtime::Value::Void) {
                            println!("{val}");
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("❌ {err}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }
    }
}

fn init_package(name: Option<String>) {
    let manifest_syn = Path::new("Package.syn");
    let manifest_toml = Path::new("syntra.toml");
    if manifest_syn.exists() || manifest_toml.exists() {
        eprintln!("❌ Error: Package manifest ('Package.syn' or 'syntra.toml') already exists in current directory.");
        exit(1);
    }

    let pkg_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my_syntra_project".to_string())
    });

    let manifest_content = format!(
        r#"// Package.syn - Syntra Package Manifest
package Config {{
    name = "{pkg_name}"
    version = "0.1.0"
    edition = "2026"

    dependencies = [
    ]
}}
"#
    );

    if let Err(e) = fs::write(manifest_syn, manifest_content) {
        eprintln!("❌ Failed to write Package.syn: {e}");
        exit(1);
    }

    let _ = fs::create_dir_all("src");
    let main_path = Path::new("src/main.syn");
    if !main_path.exists() {
        let main_content = r#"fn main(): void {
    println("Hello from Syntra!");
}
"#;
        let _ = fs::write(main_path, main_content);
    }

    let gitignore_path = Path::new(".gitignore");
    if !gitignore_path.exists() {
        let _ = fs::write(
            gitignore_path,
            "/target/\n/.syntra/\nPackage.lock\nsyntra.lock\n",
        );
    }

    println!("✨ Initialized Syntra package '{pkg_name}'");
    println!("   Created Package.syn");
    println!("   Created src/main.syn");
}

fn new_project(path: PathBuf) {
    if path.exists()
        && fs::read_dir(&path)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        eprintln!(
            "❌ Error: Target directory '{}' already exists and is not empty.",
            path.display()
        );
        exit(1);
    }

    if let Err(e) = fs::create_dir_all(&path) {
        eprintln!("❌ Failed to create directory '{}': {}", path.display(), e);
        exit(1);
    }

    let pkg_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "my_syntra_project".to_string());

    let manifest_content = format!(
        r#"// Package.syn - Syntra Package Manifest
package Config {{
    name = "{pkg_name}"
    version = "0.1.0"
    edition = "2026"

    dependencies = [
    ]
}}
"#
    );

    let manifest_path = path.join("Package.syn");
    if let Err(e) = fs::write(&manifest_path, manifest_content) {
        eprintln!("❌ Failed to write Package.syn: {e}");
        exit(1);
    }

    let src_dir = path.join("src");
    let _ = fs::create_dir_all(&src_dir);
    let main_path = src_dir.join("main.syn");
    let main_content = r#"fn main(): void {
    println("Hello from Syntra!");
}
"#;
    let _ = fs::write(main_path, main_content);

    let gitignore_path = path.join(".gitignore");
    let _ = fs::write(
        gitignore_path,
        "/target/\n/.syntra/\nPackage.lock\nsyntra.lock\n",
    );

    println!(
        "✨ Created new Syntra project '{}' at '{}'",
        pkg_name,
        path.display()
    );
}

fn add_dependency(
    package_opt: Option<String>,
    path_opt: Option<String>,
    ver_opt: Option<String>,
    git_opt: Option<String>,
    branch_opt: Option<String>,
    tag_opt: Option<String>,
    rev_opt: Option<String>,
) {
    let manifest_path = match syntra_frontend::find_package_manifest(Path::new(".")) {
        Some(p) => p,
        None => {
            eprintln!("❌ Error: No package manifest found ('Package.syn' or 'syntra.toml'). Run 'syn init' to create a package.");
            exit(1);
        }
    };

    let package = match package_opt {
        Some(p) if !p.trim().is_empty() => p,
        _ => {
            if let Some(url) = &git_opt {
                let trimmed_url = url.trim_end_matches('/').trim_end_matches(".git");
                let name = trimmed_url.rsplit('/').next().unwrap_or(trimmed_url);
                name.to_string()
            } else {
                eprintln!("❌ Error: Package name is required unless --git <url> is provided.");
                exit(1);
            }
        }
    };

    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "❌ Failed to read manifest {}: {}",
                manifest_path.display(),
                e
            );
            exit(1);
        }
    };

    let is_syn_manifest = manifest_path
        .extension()
        .map(|e| e == "syn")
        .unwrap_or(false)
        || manifest_path
            .file_name()
            .map(|n| n == "Package.syn")
            .unwrap_or(false);

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    if is_syn_manifest {
        let dep_entry = if let Some(git) = &git_opt {
            if let Some(b) = &branch_opt {
                format!("        Dependency.gitBranch(\"{package}\", \"{git}\", \"{b}\"),")
            } else if let Some(t) = &tag_opt {
                format!("        Dependency.gitTag(\"{package}\", \"{git}\", \"{t}\"),")
            } else if let Some(r) = &rev_opt {
                format!("        Dependency.gitRev(\"{package}\", \"{git}\", \"{r}\"),")
            } else {
                format!("        Dependency.git(\"{package}\", \"{git}\"),")
            }
        } else if let Some(p) = &path_opt {
            if let Some(v) = &ver_opt {
                format!("        Dependency.package(\"{package}\", \"{v}\", \"{p}\"),")
            } else {
                format!("        Dependency.path(\"{package}\", \"{p}\"),")
            }
        } else if let Some(v) = &ver_opt {
            format!("        Dependency.version(\"{package}\", \"{v}\"),")
        } else {
            format!("        Dependency.version(\"{package}\", \"0.1.0\"),")
        };

        let mut in_deps = false;
        let mut replaced = false;
        let mut insert_idx = None;

        for (i, line) in lines.iter_mut().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("dependencies") && trimmed.contains('[') {
                in_deps = true;
                continue;
            }
            if in_deps {
                if trimmed.starts_with(']') || trimmed.starts_with("];") {
                    insert_idx = Some(i);
                    break;
                }
                if trimmed.contains(&format!("\"{package}\"")) {
                    *line = dep_entry.clone();
                    replaced = true;
                    break;
                }
            }
        }

        if !replaced {
            if let Some(idx) = insert_idx {
                lines.insert(idx, dep_entry);
            } else {
                // If dependencies array not found, append it
                lines.push("    dependencies = [".to_string());
                lines.push(dep_entry);
                lines.push("    ]".to_string());
            }
        }
    } else {
        // TOML fallback
        let dep_entry = if let Some(git) = &git_opt {
            let mut parts = vec![format!("git = \"{}\"", git)];
            if let Some(b) = &branch_opt {
                parts.push(format!("branch = \"{b}\""));
            }
            if let Some(t) = &tag_opt {
                parts.push(format!("tag = \"{t}\""));
            }
            if let Some(r) = &rev_opt {
                parts.push(format!("rev = \"{r}\""));
            }
            format!("{} = {{ {} }}", package, parts.join(", "))
        } else if let Some(p) = path_opt.as_ref() {
            if let Some(v) = ver_opt.as_ref() {
                format!("{package} = {{ path = \"{p}\", version = \"{v}\" }}")
            } else {
                format!("{package} = {{ path = \"{p}\" }}")
            }
        } else if let Some(v) = ver_opt.as_ref() {
            format!("{package} = \"{v}\"")
        } else {
            format!("{package} = \"0.1.0\"")
        };

        let mut in_deps = false;
        let mut deps_found = false;
        let mut replaced = false;

        for line in &mut lines {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" {
                in_deps = true;
                deps_found = true;
                continue;
            }
            if trimmed.starts_with('[') && in_deps {
                in_deps = false;
            }

            if in_deps {
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == package {
                        *line = dep_entry.clone();
                        replaced = true;
                        break;
                    }
                }
            }
        }

        if !replaced {
            if deps_found {
                let idx = lines
                    .iter()
                    .position(|l| l.trim() == "[dependencies]")
                    .unwrap();
                lines.insert(idx + 1, dep_entry);
            } else {
                lines.push("[dependencies]".to_string());
                lines.push(dep_entry);
            }
        }
    }

    let updated = lines.join("\n") + "\n";
    if let Err(e) = fs::write(&manifest_path, updated) {
        eprintln!("❌ Failed to update {}: {}", manifest_path.display(), e);
        exit(1);
    }

    println!(
        "➕ Added dependency '{}' to {}",
        package,
        manifest_path.display()
    );

    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    if git_opt.is_some() {
        let dep = syntra_frontend::PackageDependency {
            name: package.clone(),
            version: ver_opt,
            path: path_opt,
            git: git_opt,
            branch: branch_opt,
            tag: tag_opt,
            rev: rev_opt,
        };
        println!("⬇️  Resolving remote git repository for '{package}'...");
        match syntra_frontend::resolve_git_dependency(manifest_dir, &dep) {
            Ok(cloned_path) => {
                let commit = std::process::Command::new("git")
                    .current_dir(&cloned_path)
                    .args(["rev-parse", "HEAD"])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|_| "HEAD".to_string());
                println!("📦 Cloned into '{}'", cloned_path.display());
                println!("🔒 Locked '{package}' to commit {commit}");
                install_dependencies();
            }
            Err(e) => {
                eprintln!("⚠️ Warning: git clone deferred or failed: {e}");
            }
        }
    }
}

fn remove_dependency(package: String) {
    let manifest_path = match syntra_frontend::find_package_manifest(Path::new(".")) {
        Some(p) => p,
        None => {
            eprintln!("❌ Error: No package manifest found ('Package.syn' or 'syntra.toml').");
            exit(1);
        }
    };

    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "❌ Failed to read manifest {}: {}",
                manifest_path.display(),
                e
            );
            exit(1);
        }
    };

    let is_syn_manifest = manifest_path
        .extension()
        .map(|e| e == "syn")
        .unwrap_or(false)
        || manifest_path
            .file_name()
            .map(|n| n == "Package.syn")
            .unwrap_or(false);

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut removed = false;

    if is_syn_manifest {
        let mut in_deps = false;
        lines.retain(|line| {
            let trimmed = line.trim();
            if trimmed.contains("dependencies") && trimmed.contains('[') {
                in_deps = true;
                return true;
            }
            if in_deps {
                if trimmed.starts_with(']') || trimmed.starts_with("];") {
                    in_deps = false;
                    return true;
                }
                if trimmed.contains(&format!("\"{package}\"")) {
                    removed = true;
                    return false;
                }
            }
            true
        });
    } else {
        let mut in_deps = false;
        lines.retain(|line| {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" {
                in_deps = true;
                return true;
            }
            if trimmed.starts_with('[') && in_deps {
                in_deps = false;
            }

            if in_deps {
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == package {
                        removed = true;
                        return false;
                    }
                }
            }
            true
        });
    }

    if !removed {
        println!(
            "⚠️ Dependency '{}' was not found in {}",
            package,
            manifest_path.display()
        );
        return;
    }

    let updated = lines.join("\n") + "\n";
    if let Err(e) = fs::write(&manifest_path, updated) {
        eprintln!("❌ Failed to update {}: {}", manifest_path.display(), e);
        exit(1);
    }

    println!(
        "➖ Removed dependency '{}' from {}",
        package,
        manifest_path.display()
    );
}

fn install_dependencies() {
    let manifest_path = match syntra_frontend::find_package_manifest(Path::new(".")) {
        Some(p) => p,
        None => {
            eprintln!("❌ Error: No package manifest found ('Package.syn' or 'syntra.toml'). Run 'syn init' first.");
            exit(1);
        }
    };

    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "❌ Failed to read manifest {}: {}",
                manifest_path.display(),
                e
            );
            exit(1);
        }
    };

    let manifest = syntra_frontend::PackageManifest::parse(&content);
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let packages_dir = manifest_dir.join(".syntra").join("packages");
    let _ = fs::create_dir_all(&packages_dir);

    // Resolve remote git dependencies if any
    for dep in &manifest.dependencies {
        if dep.git.is_some() {
            if let Err(e) = syntra_frontend::resolve_git_dependency(manifest_dir, dep) {
                eprintln!(
                    "⚠️ Warning: git clone deferred or failed for '{}': {}",
                    dep.name, e
                );
            }
        }
    }

    let is_syn = manifest_path
        .file_name()
        .map(|n| n == "Package.syn")
        .unwrap_or(false);
    let primary_lock = if is_syn {
        "Package.lock"
    } else {
        "syntra.lock"
    };
    let lock_path = manifest_dir.join(primary_lock);

    // Verify existing lockfile integrity if present
    if lock_path.exists() {
        let existing = match fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Error reading lockfile {}: {e}", lock_path.display());
                exit(1);
            }
        };

        if let Err(mismatches) =
            package::verify_locked_dependencies(manifest_dir, &manifest, &existing)
        {
            for m in &mismatches {
                eprintln!("❌ Security Error: {m}");
            }
            eprintln!("❌ Dependency verification failed! Run 'syn lock' to update if changes were intentional.");
            exit(1);
        }
        println!("✅ Verified dependencies against {primary_lock}");
        let _ = package::protect_package_store_readonly(&packages_dir);
    } else {
        let (lock_content, resolved_count) =
            match package::generate_lock_content(manifest_dir, &manifest) {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("❌ Error resolving dependencies: {e}");
                    exit(1);
                }
            };
        if let Err(e) =
            package::write_lockfile_atomic(manifest_dir, primary_lock, &lock_content, is_syn)
        {
            eprintln!("❌ Failed to write {primary_lock} atomically: {e}");
            exit(1);
        }

        println!(
            "📦 Resolved {} dependenc{}.",
            resolved_count,
            if resolved_count == 1 { "y" } else { "ies" }
        );
        let _ = package::protect_package_store_readonly(&packages_dir);
        println!("🔒 Generated {primary_lock}");
    }
}

fn analyze_file(file: &PathBuf) {
    let filename = file.display().to_string();
    println!("🔍 Performing Whole-Program Analysis for '{filename}'...\n");

    let (program, source) = load_program_or_exit(file);
    if let Err(diagnostics) = syntra_frontend::check(&program) {
        eprintln!("❌ Type Error:\n");
        for diag in diagnostics {
            diag.emit(&filename, &source);
        }
        exit(1);
    }

    let air_module = match syntra_air::lower_and_verify(&program) {
        Ok(m) => m,
        Err(diagnostics) => {
            eprintln!("❌ AIR Verification Error:\n");
            for diag in diagnostics {
                eprintln!("[{}] {}", diag.code, diag.message);
            }
            exit(1);
        }
    };

    let report = syntra_analysis::analyze_module(&air_module);
    println!("{}", report.print_summary());
}

fn experiment_file(file: &PathBuf, iterations: usize) {
    let filename = file.display().to_string();
    println!("⚡ Launching Syntra Experimental Compilation Engine (SECE)...");
    println!("   Target: {filename}");
    println!("   Benchmark Iterations: {iterations}\n");

    let (program, source) = load_program_or_exit(file);
    if let Err(diagnostics) = syntra_frontend::check(&program) {
        eprintln!("❌ Type Error:\n");
        for diag in diagnostics {
            diag.emit(&filename, &source);
        }
        exit(1);
    }

    let air_module = match syntra_air::lower_and_verify(&program) {
        Ok(m) => m,
        Err(diagnostics) => {
            eprintln!("❌ AIR Verification Error:\n");
            for diag in diagnostics {
                eprintln!("[{}] {}", diag.code, diag.message);
            }
            exit(1);
        }
    };

    match syntra_sandbox::ExperimentalCompilationEngine::run_experiment(
        &filename,
        &air_module,
        iterations,
    ) {
        Ok(report) => {
            println!("{}", report.format_report());
        }
        Err(err) => {
            eprintln!("❌ SECE Experiment failed: {err}");
            exit(1);
        }
    }
}

fn approve_candidate(candidate_id: String, dir_opt: Option<PathBuf>) {
    let dir = dir_opt.unwrap_or_else(|| PathBuf::from("."));
    match syntra_sandbox::ExperimentalCompilationEngine::approve_candidate(&candidate_id, &dir) {
        Ok(path) => {
            println!("✅ Successfully approved strategy '{candidate_id}'!");
            println!("🔒 Manifest recorded at: {}", path.display());
        }
        Err(err) => {
            eprintln!("❌ Approval failed: {err}");
            exit(1);
        }
    }
}

fn profile_file(file: &PathBuf, output: Option<PathBuf>, lock: bool, args: Vec<String>) {
    let filename = file.display().to_string();
    println!("⏱️ Profiling runtime execution for '{filename}'...\n");

    let (program, source) = load_program_or_exit(file);
    if let Err(diagnostics) = syntra_frontend::check(&program) {
        eprintln!("❌ Type Error:\n");
        for diag in diagnostics {
            diag.emit(&filename, &source);
        }
        exit(1);
    }

    match syntra_runtime::run_with_profiling(&program, args) {
        Ok((result, profile)) => {
            println!("{}", profile.format_table());

            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
            let profiles_dir = Path::new(".syntra").join("profiles");
            let _ = fs::create_dir_all(&profiles_dir);
            let profile_path = profiles_dir.join(format!("{stem}.profile.json"));
            let json_str = profile.to_json();
            let _ = fs::write(&profile_path, &json_str);
            println!("💾 Profile snapshot saved: {}", profile_path.display());

            if lock {
                let synlock_str = profile.to_synlock();
                let synlock_path = Path::new("profile.synlock");
                if let Err(e) = fs::write(synlock_path, &synlock_str) {
                    eprintln!("❌ Failed to write profile.synlock: {e}");
                } else {
                    println!(
                        "🔒 Generated deterministic profile lockfile: {}",
                        synlock_path.display()
                    );
                }
            }

            if let Some(out_p) = output {
                let _ = fs::write(&out_p, &json_str);
                println!("💾 Profile exported to: {}", out_p.display());
            }

            // Demonstrate PGO feedback loop into Whole-Program Analysis (Subsystems 9 -> 3)
            if let Ok(air_module) = syntra_air::lower_and_verify(&program) {
                let report =
                    syntra_analysis::analyze_module_with_profile(&air_module, Some(profile));
                println!("\n{}", report.print_summary());
            }

            match result {
                syntra_runtime::Value::Void => {}
                syntra_runtime::Value::Int(code) => {
                    println!("\n[Program exited with code: {code}]");
                }
                other => {
                    println!("\n[Program returned: {other}]");
                }
            }
        }
        Err(err) => {
            eprintln!("❌ Runtime Error during profiling: {err}");
            exit(1);
        }
    }
}

fn tune_file(file: &PathBuf, iterations: usize, output: Option<PathBuf>, args: Vec<String>) {
    let filename = file.display().to_string();
    println!("═════════════════════════════════════════════════════════════════════════");
    println!("       Syntra Autonomous PGO Tuning Engine (syn tune)                    ");
    println!("═════════════════════════════════════════════════════════════════════════");
    println!("🎯 Target Application: {filename}");
    println!("🔄 Sandbox Iterations: {iterations}\n");

    // Phase 1: Runtime Profiling & Lockfile Generation (Subsystem 9 SAR)
    println!("--- [Phase 1/4] Profiling Runtime Workload (SAR Telemetry) ---");
    let (program, source) = load_program_or_exit(file);
    if let Err(diagnostics) = syntra_frontend::check(&program) {
        eprintln!("❌ Type Error:\n");
        for diag in diagnostics {
            diag.emit(&filename, &source);
        }
        exit(1);
    }

    match syntra_runtime::run_with_profiling(&program, args) {
        Ok((_, profile)) => {
            println!(
                "✅ Execution profiling completed ({} functions tracked).",
                profile.functions.len()
            );
            let synlock_str = profile.to_synlock();
            let synlock_path = Path::new("profile.synlock");
            if let Err(e) = fs::write(synlock_path, &synlock_str) {
                eprintln!("⚠️ Warning: could not write profile.synlock: {e}");
            } else {
                println!("🔒 Generated canonical profile lockfile: profile.synlock");
            }
        }
        Err(e) => {
            eprintln!("❌ Profiling run failed: {e}");
            exit(1);
        }
    }

    // Phase 2: Ephemeral Sandbox Exploration & Differential Verification (Subsystem 7 Sandbox)
    println!("\n--- [Phase 2/4] Sandbox Differential Testing & Pareto Ranking ---");
    let air_module = match syntra_air::lower_and_verify(&program) {
        Ok(m) => m,
        Err(diags) => {
            for d in diags {
                eprintln!("[{}] {}", d.code, d.message);
            }
            exit(1);
        }
    };

    match syntra_sandbox::ExperimentalCompilationEngine::run_experiment(
        &filename,
        &air_module,
        iterations,
    ) {
        Ok(rep) => {
            println!("{}", rep.format_report());
        }
        Err(err) => {
            eprintln!("❌ Sandbox experiment failed: {err}");
            exit(1);
        }
    }

    // Phase 3: Empirical Model Retraining & Calibration (Subsystem 10 SLS)
    println!("--- [Phase 3/4] Syntra Learning System (SLS Model Calibration) ---");
    let dataset_path = Path::new(".syntra/datasets/training_records.jsonl");
    if dataset_path.is_file() {
        if let Ok(dataset) = syntra_intelligence::OptimizationDataset::load_from_file(dataset_path)
        {
            if let Some(trained_model) =
                syntra_intelligence::ModelPredictor::train_from_dataset(&dataset)
            {
                println!(
                    "🧠 Model updated: {} (Digest: {})",
                    trained_model.metadata.version, trained_model.metadata.digest
                );
                println!(
                    "📊 Accuracy: {:.1}%, Calibration Error: {:.3}, RMSE: {:.2}%",
                    trained_model.metadata.evaluation_accuracy * 100.0,
                    trained_model.metadata.calibration_error,
                    trained_model.metadata.rmse
                );
                let mut reg = syntra_intelligence::VersionedModelRegistry::new();
                let audit_path = Path::new(".syntra/models/audit.log");
                let reg_path = Path::new(".syntra/models/registry.json");
                reg.register_model(trained_model, Some(audit_path));
                let _ = reg.save_registry_manifest(reg_path);
            }
        }
    }

    // Phase 4: Final Release Native Build with Verified PGO (Subsystem 5 Optimizer)
    println!("\n--- [Phase 4/4] Compiling Optimized Release Executable (PGO Enforced) ---");
    build_native_binary(
        file,
        output,
        None,
        true,
        Some(PathBuf::from("profile.synlock")),
    );

    println!("\n═════════════════════════════════════════════════════════════════════════");
    println!("✅ AUTONOMOUS TUNING COMPLETE: Release binary optimized with empirical PGO!");
    println!("═════════════════════════════════════════════════════════════════════════");
}

pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file, sar, args }) => {
            execute_file(&file, args, sar);
        }
        Some(Commands::Build {
            file,
            output,
            target,
            pgo,
            profile,
        }) => {
            build_native_binary(&file, output, target, pgo, profile);
        }
        Some(Commands::Check {
            file,
            dump_ast,
            dump_air,
        }) => {
            check_file(&file, dump_ast, dump_air);
        }
        Some(Commands::Air { file }) => {
            let (program, _source) = load_program_or_exit(&file);
            emit_air(&program);
        }
        Some(Commands::Opt { file }) => {
            optimize_file(&file);
        }
        Some(Commands::Plan { file }) => {
            plan_file(&file);
        }
        Some(Commands::Manifest { file }) => {
            manifest_file(&file);
        }
        Some(Commands::Doc {
            file,
            format,
            output,
        }) => {
            generate_docs(&file, &format, output);
        }
        Some(Commands::Init { name }) => {
            init_package(name);
        }
        Some(Commands::New { path }) => {
            new_project(path);
        }
        Some(Commands::Add {
            package,
            path,
            version,
            git,
            branch,
            tag,
            rev,
        }) => {
            add_dependency(package, path, version, git, branch, tag, rev);
        }
        Some(Commands::Remove { package }) => {
            remove_dependency(package);
        }
        Some(Commands::Install) => {
            install_dependencies();
        }
        Some(Commands::Lock { check }) => {
            package::lock_dependencies(check);
        }
        Some(Commands::Pack { out, vendor }) => {
            package::pack_project(out, vendor);
        }
        Some(Commands::Fmt { file, check }) => {
            fmt::format_files(file, check);
        }
        Some(Commands::Lsp) => {
            lsp::run_server();
        }
        Some(Commands::Analyze { file }) => {
            analyze_file(&file);
        }
        Some(Commands::Experiment { file, iterations }) => {
            experiment_file(&file, iterations);
        }
        Some(Commands::Approve { candidate_id, dir }) => {
            approve_candidate(candidate_id, dir);
        }
        Some(Commands::Profile {
            file,
            output,
            lock,
            args,
        }) => {
            profile_file(&file, output, lock, args);
        }
        Some(Commands::Tune {
            file,
            iterations,
            output,
            args,
        }) => {
            tune_file(&file, iterations, output, args);
        }
        Some(Commands::Debug { file, dap, args }) => {
            if dap {
                debug::run_dap_server();
            } else if let Some(file) = file {
                if let Err(err) = debug::run_cli_debugger(file.to_str().unwrap_or(""), args) {
                    eprintln!("❌ Debugger error: {err}");
                    exit(1);
                }
            } else {
                eprintln!(
                    "❌ Error: A source file is required for debugging unless --dap is specified."
                );
                eprintln!("Usage: syn debug <file.syn> [args...] OR syn debug --dap");
                exit(1);
            }
        }
        Some(Commands::Learn { command }) => {
            handle_learn_command(command);
        }
        Some(Commands::Mail { command }) => {
            handle_mail_command(command);
        }
        None => {
            if let Some(file) = cli.file {
                execute_file(&file, cli.args, false);
            } else {
                start_repl();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_check_and_run_subcommands() {
        let check_args = ["syn", "check", "test.syn", "--dump-ast"];
        let cli = Cli::try_parse_from(check_args).expect("Parse check command");
        match cli.command {
            Some(Commands::Check {
                file,
                dump_ast,
                dump_air,
            }) => {
                assert_eq!(file, PathBuf::from("test.syn"));
                assert!(dump_ast);
                assert!(!dump_air);
            }
            _ => panic!("Expected Commands::Check"),
        }

        let run_args = ["syn", "run", "--sar", "test.syn", "arg1", "arg2"];
        let cli_run = Cli::try_parse_from(run_args).expect("Parse run command");
        match cli_run.command {
            Some(Commands::Run { file, sar, args }) => {
                assert_eq!(file, PathBuf::from("test.syn"));
                assert!(sar);
                assert_eq!(args, vec!["arg1", "arg2"]);
            }
            _ => panic!("Expected Commands::Run"),
        }
    }

    #[test]
    fn test_cli_parse_build_and_sir_aliases() {
        let build_args = ["syn", "build", "main.syn", "-o", "out_bin", "--pgo"];
        let cli_b = Cli::try_parse_from(build_args).expect("Parse build command");
        match cli_b.command {
            Some(Commands::Build {
                file, output, pgo, ..
            }) => {
                assert_eq!(file, PathBuf::from("main.syn"));
                assert_eq!(output, Some(PathBuf::from("out_bin")));
                assert!(pgo);
            }
            _ => panic!("Expected Commands::Build"),
        }

        // Test SIR command alias for AIR
        let sir_args = ["syn", "sir", "main.syn"];
        let cli_sir = Cli::try_parse_from(sir_args).expect("Parse sir alias");
        match cli_sir.command {
            Some(Commands::Air { file }) => {
                assert_eq!(file, PathBuf::from("main.syn"));
            }
            _ => panic!("Expected Commands::Air for 'sir' alias"),
        }
    }

    #[test]
    fn test_cli_parse_approve_and_package() {
        let approve_args = ["syn", "approve", "cand_99_test", "--dir", ".syntra"];
        let cli_appr = Cli::try_parse_from(approve_args).expect("Parse approve command");
        match cli_appr.command {
            Some(Commands::Approve { candidate_id, dir }) => {
                assert_eq!(candidate_id, "cand_99_test");
                assert_eq!(dir, Some(PathBuf::from(".syntra")));
            }
            _ => panic!("Expected Commands::Approve"),
        }

        let lock_args = ["syn", "lock", "--check"];
        let cli_pkg = Cli::try_parse_from(lock_args).expect("Parse lock");
        match cli_pkg.command {
            Some(Commands::Lock { check }) => {
                assert!(check);
            }
            _ => panic!("Expected Commands::Lock"),
        }
    }
}
