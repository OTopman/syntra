# Syntra Phase 1 Final Security Acceptance Audit
# Document 06: Architectural Improvement Register

**Audited Component:** Package Integrity & Supply-Chain Security Architecture  
**Audited Revision:** `826b16de4e20fc2bcb4fdea4f2257bff88ed4ea4`  
**Auditor:** Principal Compiler Security Engineer & Systems Runtime Auditor  
**Date:** 2026-11-24  

---

## 1. Overview & Purpose

This register formally logs all residual security findings, architectural boundary considerations, and future hardening items identified during the independent Phase 1 Final Security Acceptance Audit. 

In accordance with compiler security governance, findings are categorized using standard CVSS / severity ratings (Critical, High, Medium, Low, Informational) and provided with concrete reproduction mechanisms, threat models, and explicit remediation steps.

---

## 2. Security Findings Register

### FIND-PH1-AUDIT-01: Debugger CLI and DAP Entry Point Verification Bypass

- **Severity:** **MEDIUM** (CVSS: 5.3 - AV:L/AC:L/PR:N/UI:R/S:U/C:L/I:L/A:L)
- **Component:** `crates/cli/src/debug.rs` (`cmd_debug` line 21, `cmd_dap` line 471)
- **Target Subcommand:** `syn debug <file>`, `syn debug --dap`
- **Description:**
  The interactive debugger (`syn debug`) and the Debug Adapter Protocol server (`syn debug --dap`) instantiate `syntra_frontend::loader::ModuleLoader::new()` directly and call `.load_program(path)` without executing `package::verify_package_consumption(path)`.
- **Vulnerability Mechanics:**
  While 11 compiler entry points (`build`, `run`, `check`, etc.) route through `load_program_or_exit` to enforce SHA-256 tree verification before AST loading, `debug.rs` bypasses the verification gate entirely.
- **Proof of Concept / Threat Impact:**
  1. A project has dependency `libcrypto` pinned in `Package.lock`.
  2. An attacker or malicious local tool modifies `.syntra/packages/libcrypto/1.0.0/src/lib.syn` to execute unauthorized host operations.
  3. Running `syn run main.syn` or `syn build main.syn` immediately aborts with:
     `"❌ Security Error: Dependency verification failed at compiler consumption time"`.
  4. Running `syn debug main.syn` or launching an IDE debugging session in VS Code completely bypasses verification, loads the tampered AST, and executes the payload.
- **Recommended Remediation:**
  Refactor `cmd_debug` and `cmd_dap` to invoke `package::verify_package_consumption(Path::new(file_path))` before instantiating the module loader, returning an error if verification fails.
- **Tracking Status:** **RESOLVED (VERIFIED)** — Gated in `crates/cli/src/debug.rs` (lines 20-32, lines 471-482) and verified by `test_debugger_cli_and_dap_gated_by_package_verification`.

---

### FIND-PH1-AUDIT-02: Non-Atomic Lockfile File Replacement and Lack of Advisory File Locking

- **Severity:** **LOW** (CVSS: 3.3 - AV:L/AC:L/PR:L/UI:N/S:U/C:N/I:L/A:L)
- **Component:** `crates/cli/src/package.rs` (`lock_packages`, line 997, 1002), `crates/cli/src/lib.rs`
- **Description:**
  `Package.lock` and `syntra.lock` updates are executed via standard `fs::write(&lock_path, &lock_content)` without write-rename atomicity or advisory file locking.
- **Vulnerability Mechanics:**
  1. In-place truncation: `fs::write` opens the target with `O_TRUNC | O_WRONLY`. If the process terminates abruptly (power loss, SIGKILL, crash), the lockfile is truncated to 0 bytes or left in a partial state. Subsequent tool invocations fail.
  2. Concurrent race condition: If an IDE language server (`syn check`) and a developer CLI command (`syn install` / `syn lock`) execute concurrently on the same workspace, concurrent writes to `Package.lock` can interleave and corrupt lockfile syntax.
- **Proof of Concept / Threat Impact:**
  A sudden process termination during `syn lock` leaves an empty `Package.lock`. Subsequent `syn build` invocations abort with lockfile parse errors.
- **Recommended Remediation:**
  1. Write updated lockfile content to a temporary sibling file (`Package.lock.tmp.<pid>`).
  2. Flush and sync (`file.sync_all()`).
  3. Atomically overwrite the destination via `fs::rename(tmp_path, lock_path)`.
  4. Acquire an advisory lock (`fs2::FileExt::lock_exclusive` or equivalent) on `Package.lock` during write operations.
- **Tracking Status:** **RESOLVED (VERIFIED)** — Implemented `atomic_write_file` and `write_lockfile_atomic` in `crates/cli/src/package.rs` with `sync_all` and atomic rename; verified by `test_atomic_lockfile_replacement_and_recovery`.

---

### FIND-PH1-AUDIT-03: OS Permission Model Limitations for Package Store Immutable Guarantees

- **Severity:** **INFORMATIONAL / ARCHITECTURAL BOUNDARY**
- **Component:** `crates/cli/src/package.rs` (`protect_package_store_readonly`, lines 316-347)
- **Description:**
  Hardening package directories to `0o555` and files to `0o444` provides defense-in-depth against accidental modifications, but does not provide an immutable cryptographic snapshot against same-UID concurrent processes.
- **Vulnerability Mechanics:**
  In POSIX discretionary access control, the owner of a file (the user running `syn`) has permission to alter file mode bits (`chmod u+w`). A concurrent malicious process running under the same user UID can alter files in `.syntra/packages/` during the microsecond window between `verify_package_consumption` and compiler `fs::read_to_string` file ingestion.
- **Architectural Boundary Clarification:**
  Eliminating TOCTOU host races cannot be achieved purely through userspace file permissions. True immutable snapshots require kernel-enforced mount namespaces, read-only loopback devices (`dm-verity`), or complete execution sandboxing. This boundary is the primary design justification for **Phase 2 (SECE Sandbox Isolation)**.
- **Recommended Action:**
  Formally define Phase 1's trust model as "supply-chain pre-consumption integrity" and rely on Phase 2's kernel namespace sandboxing to isolate process execution.
- **Tracking Status:** Closed as Architectural Boundary (Deferred to Phase 2).

---

### FIND-PH1-AUDIT-04: Path Delimiter Framing in Canonical Entry Serialization

- **Severity:** **LOW** (CVSS: 2.5 - AV:L/AC:H/PR:L/UI:N/S:U/C:N/I:L/A:N)
- **Component:** `crates/cli/src/package.rs` (`hash_directory_tree_sha256`, lines 282-285)
- **Description:**
  Canonical entry serialization records relative paths using newline delimiters (`path:<rel_path>\n`), whereas file contents use explicit length prefixing (`size:<len>\n`).
- **Vulnerability Mechanics:**
  On POSIX filesystems, directory and file names may legally contain newline characters (`\n`). If a package author crafts a filename containing embedded newlines and header tokens matching the entry framing protocol, it could theoretically produce a collision with a crafted separate entry.
- **Recommended Remediation:**
  1. Reject any path containing newline characters in `collect_canonical_entries` (`if rel_path_str.contains('\n') { return Err(...) }`).
  2. Alternatively, record path length explicitly (`path_len:<len>\npath:<bytes>\n`).
- **Tracking Status:** **RESOLVED (VERIFIED)** — Input validation rejects `\n`/`\r`/`\0` in paths and symlinks; length-prefixed framing implemented for all variable fields in `hash_directory_tree_sha256`; verified by `test_canonical_tree_hashing_length_prefixed_framing`.

---

### FIND-PH1-AUDIT-05: Remote Registry Package Cryptographic Envelope Signatures

- **Severity:** **INFORMATIONAL**
- **Component:** Package Distribution Architecture (`crates/cli/src/package.rs`)
- **Description:**
  The current package manager verifies dependencies once unpacked into `.syntra/packages/<name>/<version>`. There is currently no asymmetric signature verification (e.g., Ed25519 / Minisign / Sigstore) over packaged `.synpkg` archive distribution envelopes.
- **Architectural Context:**
  Phase 1 focuses on content-addressable integrity and deterministic lockfile pinning. As Syntra establishes a public or federated package registry, distribution envelopes should include author signatures to verify origin authenticity.
- **Recommended Remediation:**
  Design a public-key signing infrastructure for `.synpkg` archives when building the remote registry transport client in Phase 3/4.
- **Tracking Status:** Logged for future Registry Distribution Roadmap (Phase 4).

---

## 3. Improvement Roadmap & Ownership Summary

| Finding ID | Severity | Status | Resolution | Blocker for Phase 2? |
| :--- | :--- | :--- | :--- | :--- |
| **FIND-PH1-AUDIT-01** (Debugger Bypass) | Medium | **RESOLVED (VERIFIED)** | Gated CLI debugger & DAP server | **NO (Resolved)** |
| **FIND-PH1-AUDIT-02** (Lockfile Atomicity) | Low | **RESOLVED (VERIFIED)** | Atomic temporary file & rename | **NO (Resolved)** |
| **FIND-PH1-AUDIT-03** (TOCTOU Host Boundary)| Informational | **ARCHITECTURAL BOUNDARY** | Bounded to **Phase 2 (SECE Sandbox)** | **NO (Phase 2 Goal)** |
| **FIND-PH1-AUDIT-04** (Path Delimiter Framing)| Low | **RESOLVED (VERIFIED)** | Length-prefixed framing & input validation | **NO (Resolved)** |
| **FIND-PH1-AUDIT-05** (Registry Signatures) | Informational | **DOCUMENTED BOUNDARY** | Deferred to Phase 4 (Registry Client) | **NO** |

**Conclusion:** All three actionable technical findings (`FIND-PH1-AUDIT-01`, `FIND-PH1-AUDIT-02`, `FIND-PH1-AUDIT-04`) have been fully remediated and verified with 31 passing adversarial tests and 118 workspace tests. The two remaining items are documented architectural boundaries for subsequent phases.
