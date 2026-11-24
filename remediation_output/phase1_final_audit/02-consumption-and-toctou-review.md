# Syntra Phase 1 Final Security Acceptance Audit
# Document 02: Consumption Gate & TOCTOU Threat Review

**Target Component:** Compiler Pre-Consumption Verification & Package Store Protections  
**Source Files:** `crates/cli/src/package.rs` (`verify_package_consumption`, `protect_package_store_readonly`), `crates/cli/src/lib.rs` (`load_program_or_exit`)  
**Audited Revision:** `826b16de4e20fc2bcb4fdea4f2257bff88ed4ea4`  
**Auditor:** Principal Compiler Security Engineer & Supply-Chain Security Architect  
**Status:** **ACCEPTED WITH LIMITATIONS** (Technical Audit Passed, 1 Informational Architectural Boundary Finding)

---

## 1. Executive Summary & Audit Scope

A major deficiency identified in earlier reviews was that package verification occurred exclusively during package management commands (`syn install`, `syn lock`), but was never evaluated when the compiler actually compiled or executed user code (`syn build`, `syn run`, `syn check`). If a malicious local process, compromised dependency, or unauthorized script modified files in the package cache (`.syntra/packages/...`) *after* installation, the compiler would blindly consume the compromised AST and emit native machine code containing malicious instructions.

This document reviews:
1. The architecture and enforcement of the compiler pre-consumption verification gate.
2. The package store read-only permission hardening (`protect_package_store_readonly`).
3. An in-depth threat model of Time-of-Check to Time-of-Use (TOCTOU) risks across operating system boundaries.
4. The architectural delineation between compiler-level pre-consumption integrity and kernel-level execution containment (Phase 2 SECE sandbox).

---

## 2. Package Lifecycle Architecture

The end-to-end package lifecycle in Syntra operates through five distinct phases:

```
[ Phase A: Manifest Declaration ]
        │  Package.syn / syntra.toml declarations
        ▼
[ Phase B: Resolution & Installation ]
        │  Fetch registry archives / clone git trees to .syntra/packages/
        │  Generate lockfile (syn lock) with SHA-256 tree hashes
        │  Apply read-only permissions (protect_package_store_readonly)
        ▼
[ Phase C: Compiler Pre-Consumption Gate ]
        │  Triggered by 'syn run', 'syn build', 'syn check', etc.
        │  Re-computes SHA-256 tree hash over disk contents
        │  Validates against Package.lock and manifest
        │  ABORTS compilation immediately on any mismatch
        ▼
[ Phase D: Module Loader AST Ingestion ]
        │  syntra_frontend::loader::ModuleLoader loads and parses source files
        ▼
[ Phase E: Compilation & Execution ]
        │  Type-checking, IR optimization, Cranelift native codegen / execution
```

---

## 3. Pre-Consumption Verification Gate Analysis

The gate is implemented in `crates/cli/src/lib.rs:247-260`:

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
    // ... loads program only if verification succeeds ...
}
```

### Verification Logic in `package::verify_package_consumption`:
1. Scans parent directories from the entry file to find the package manifest (`Package.syn` or `syntra.toml`).
2. If no manifest is found (standalone script), verification passes.
3. If a manifest is found, it locates the companion lockfile (`Package.lock` or `syntra.lock`).
   - If the lockfile is missing, compilation aborts: `"Manifest found at '...' but no lockfile exists"`.
4. Parses the lockfile using strict TOML schema validation (`Lockfile::parse`).
5. Iterates through all locked dependencies:
   - For each dependency, locates the on-disk directory in `.syntra/packages/<name>/<version>`.
   - Computes the real-time FIPS 180-4 SHA-256 canonical tree hash (`hash_directory_tree_sha256`).
   - For Git dependencies, verifies that the locked commit hash matches.
   - Compares the real-time disk hash against the locked hash.
6. Returns an aggregated list of mismatches. If any mismatch is present, the compiler aborts with exit code 1 **before** opening or parsing a single dependency source file.

---

## 4. Package Store Read-Only Permission Hardening

To mitigate tampering between installation and consumption, `protect_package_store_readonly` (`crates/cli/src/package.rs:316-347`) applies POSIX permissions across the dependency directory tree:

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let mode = if ft.is_dir() { 0o555 } else { 0o444 };
    let _ = fs::set_permissions(&p, fs::Permissions::from_mode(mode));
}
```

### Security Benefits:
- Files are marked `0444` (read-only for owner, group, and world).
- Directories are marked `0555` (read-execute, no write for owner, group, or world).
- Prevents accidental overwrites by text editors, IDE language servers, or normal development tools.
- Prevents unprivileged scripts from writing directly to files without an explicit `chmod`.

---

## 5. TOCTOU Threat Model & Residual Boundary Analysis

### FIND-PH1-AUDIT-03 (Severity: INFORMATIONAL / ARCHITECTURAL BOUNDARY):
**Finding Title:** OS Permission Model Limitations for Package Store Immutable Guarantees.

While setting permissions to `0o444` and `0o555` is a valuable hardening measure, the audit team explicitly highlights that **POSIX read-only permissions do NOT establish an immutable cryptographic store or eliminate TOCTOU races**.

### Technical Mechanics of the Race:
1. **Same-UID Privilege:** In Unix and Windows security models, the owner of a file (the developer or build user) possesses discretionary access control permissions. Any concurrent process running under the same user UID can invoke `chmod(path, 0o644)` or `rename()` without root privileges.
2. **Microsecond Inter-Syscall Window:**
   - At $t_0$, `hash_directory_tree_sha256` reads all files in `.syntra/packages/foo/` and computes SHA-256 digest `D`.
   - At $t_1$, `verify_package_consumption` validates `D == D_lock` and returns `Ok(())`.
   - At $t_2$, a concurrent malicious background process alters `.syntra/packages/foo/src/lib.syn`.
   - At $t_3$, `ModuleLoader::load_program` calls `fs::read_to_string` on `.syntra/packages/foo/src/lib.syn` and ingests the compromised code.
3. **Absence of Kernel Verification Backing:** The package store is not mounted on a cryptographic read-only filesystem (such as `dm-verity` or `fs-verity`), nor is it isolated within a private mount namespace.

---

## 6. Security Boundary Classification

To maintain precise architectural definitions, Syntra's security posture is delineated into three layers:

| Security Layer | Scope | Mechanism | Current Status in Syntra |
| :--- | :--- | :--- | :--- |
| **Layer 1: Pre-Consumption Verification** | Supply-chain integrity before compilation | Full SHA-256 tree hashing + lockfile gate in `load_program_or_exit` | **VERIFIED & ENFORCED** (Phase 1) |
| **Layer 2: Package Store Tamper Resistance** | Defense-in-depth against accidental modification | `0o444`/`0o555` file/dir permissions | **VERIFIED & ENFORCED** (Phase 1) |
| **Layer 3: Process & Filesystem Isolation** | Execution-time isolation against malicious processes | OS Sandboxing (Namespaces, Landlock, Cgroups, Seccomp) | **PHASE 2 (SECE Sandbox)** |

### Architectural Conclusion:
Supply-chain verification (Phase 1) ensures that dependencies *as stored and checked on disk* match their recorded cryptographic provenance before AST consumption.

Preventing runtime hostile races and isolating processes from interacting with the host system is the specific design remit of **Phase 2 (SECE Sandbox Isolation)**. Expecting Phase 1 package hashing to solve OS-level concurrent same-UID race attacks is an improper mixing of security domains.

---

## 7. Verification Evidence

Adversarial tests in `crates/cli/tests/test_package_integrity.rs`:
- `test_verify_package_consumption_clean_tree_succeeds`: Confirmed valid dependencies pass the consumption gate.
- `test_verify_package_consumption_tampered_tree_fails`: Confirmed modifying a file in `.syntra/packages/` causes compilation to abort with an explicit tampering error.
- `test_verify_package_consumption_missing_lockfile_fails`: Confirmed missing lockfiles abort compilation.
- `test_protect_package_store_readonly_enforces_permissions`: Confirmed read-only mode bits are set across the tree.

**Audit Assessment:** The consumption-time gate is robustly enforced across compiler commands. The TOCTOU limitations of filesystem permissions are formally documented in the architectural improvement register as an expected boundary to be addressed by Phase 2 sandboxing.
