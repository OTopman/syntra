# Syntra Phase 1 Final Security Acceptance Audit
# Document 04: Compiler Entry Point & Command Surface Coverage

**Target Component:** CLI Subcommand Dispatch & AST Loading Pipeline  
**Source Files:** `crates/cli/src/lib.rs`, `crates/cli/src/debug.rs`  
**Audited Revision:** `826b16de4e20fc2bcb4fdea4f2257bff88ed4ea4`  
**Auditor:** Principal Compiler Security Engineer & Systems Runtime Auditor  
**Status:** **ACCEPTED WITH LIMITATIONS** (1 Medium-Severity Residual Finding: Debugger Entry Point Bypass)

---

## 1. Executive Summary & Audit Scope

A security gate is only as strong as its enforcement boundary. In compiler supply-chain security, ensuring that package integrity checks run during `syn install` or `syn lock` is necessary but insufficient. Every CLI entry point that ingests source code, constructs a module dependency graph, parses syntax trees, or generates machine instructions must systematically route through package verification.

This document presents a comprehensive audit of all Syntra CLI subcommands to determine:
1. Which commands trigger the compiler pipeline and source loading.
2. Whether every compiler entry point enforces `verify_package_consumption`.
3. The exact mechanics, line references, and threat impact of any identified bypasses.

---

## 2. Comprehensive CLI Subcommand Matrix

The table below maps all 23 CLI subcommands defined in `crates/cli/src/lib.rs` and their enforcement status:

| Command | Handler Function | Ingests Dependencies? | Enforcement Mechanism | Security Status |
| :--- | :--- | :--- | :--- | :--- |
| `syn run` | `execute_file` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn build` | `build_native_binary` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn check` | `typecheck_file` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn air` | `emit_air` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn opt` | `optimize_file` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn plan` | `plan_execution` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn manifest` | `emit_manifest` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn tune` | `tune_pipeline` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn experiment`| `run_experiment` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn profile` | `profile_file` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn analyze` | `analyze_program` | Yes | `load_program_or_exit` | **ENFORCED** |
| `syn debug` (interactive) | `cmd_debug` (`debug.rs:21`) | Yes | Direct `ModuleLoader` | **BYPASS (FIND-PH1-AUDIT-01)** |
| `syn debug --dap` (DAP) | `cmd_dap` (`debug.rs:471`) | Yes | Direct `ModuleLoader` | **BYPASS (FIND-PH1-AUDIT-01)** |
| `syn install` | `package::install_packages` | N/A (Manager) | Computes SHA-256 + applies read-only | **ENFORCED** |
| `syn lock` | `package::lock_packages` | N/A (Manager) | Canonical tree verification | **ENFORCED** |
| `syn pack` | `package::pack_project` | N/A (Archiver) | Checks manifest & files | **SECURE** |
| `syn test` | `test_runner` | Yes (Test files) | Delegates to compiler harness | **ENFORCED** |
| `syn fmt` | `format_file` | Single file only | AST formatting only | N/A |
| `syn doc` | `generate_docs` | Source comments | Documentation parser | N/A |
| `syn init` | `init_project` | N/A | Scaffolds template | N/A |
| `syn new` | `new_project` | N/A | Scaffolds directory | N/A |
| `syn clean` | `clean_artifacts` | N/A | Removes `target/` | N/A |
| `syn repl` | `repl::start` | Interactive expr | Sandboxed interactive eval | N/A |

---

## 3. Analysis of Enforced Pipeline (`load_program_or_exit`)

For all 11 core compilation and analysis commands (`run`, `build`, `check`, `air`, `opt`, `plan`, `manifest`, `tune`, `experiment`, `profile`, `analyze`), execution routes through a single centralized gateway in `crates/cli/src/lib.rs:247-278`:

```rust
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
    // ...
}
```

This single bottleneck provides total verification coverage across all primary user workflows.

---

## 4. Deep Inspection of Debugger Entry Point Bypass

### FIND-PH1-AUDIT-01 (Severity: MEDIUM):
**Finding Title:** Debugger CLI and DAP Server Entry Point Verification Bypass.

### Affected Code Locations:
1. `crates/cli/src/debug.rs:21-25`:
```rust
pub fn cmd_debug(file_path: &str, _break_at: Option<String>) -> Result<(), String> {
    // ...
    let mut loader = syntra_frontend::loader::ModuleLoader::new();
    let path = Path::new(file_path);
    let program = loader
        .load_program(path)
        .map_err(|diags| format!("Failed to load program: {diags:?}"))?;
    // ...
}
```

2. `crates/cli/src/debug.rs:471-474`:
```rust
pub fn cmd_dap() -> Result<(), String> {
    // ...
    let mut loader = syntra_frontend::loader::ModuleLoader::new();
    let path = Path::new(program_path);
    if let Ok(prog) = loader.load_program(path) {
        // ...
    }
}
```

### Vulnerability Mechanics & Threat Impact:
- **Root Cause:** The debugger and Debug Adapter Protocol (DAP) implementation in `crates/cli/src/debug.rs` was written as an independent subsystem. It bypasses `load_program_or_exit` and instantiates `syntra_frontend::loader::ModuleLoader` directly.
- **Attack Scenario:**
  1. A project includes dependency `cryptolib = "1.0.0"`.
  2. An attacker or malicious local tool modifies `.syntra/packages/cryptolib/1.0.0/src/lib.syn` to inject malicious backdoor routines.
  3. If the developer runs `syn build main.syn` or `syn run main.syn`, the consumption gate halts execution with exit code 1.
  4. However, if the developer launches the debugger via `syn debug main.syn` or through an IDE debug session using VS Code (`syn debug --dap`), the debugger **loads the tampered dependency, parses its AST, evaluates its code, and executes it without any verification**.
- **Impact Assessment:** Medium. The debugger allows arbitrary code execution of tampered packages that would be rejected by all standard compilation commands.

### Remediation Recommendation:
Update `crates/cli/src/debug.rs` to call `package::verify_package_consumption(path)` prior to instantiating `ModuleLoader`, or refactor `cmd_debug` and `cmd_dap` to load programs via `load_program_or_exit`.

---

## 5. Audit Conclusion

The compiler pre-consumption gate is 100% effective across all standard build, run, analysis, and optimization tools (11/11 commands). The single architectural gap exists in the specialized debugger subsystem (`debug.rs`), which must be gated in subsequent maintenance. This finding does not invalidate the package integrity architecture, but is formally logged in `06-architectural-improvement-register.md` as `FIND-PH1-AUDIT-01`.
