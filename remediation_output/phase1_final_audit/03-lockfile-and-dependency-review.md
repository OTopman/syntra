# Syntra Phase 1 Final Security Acceptance Audit
# Document 03: Lockfile Specification & Dependency Verification Review

**Target Component:** Lockfile Deserialization, Schema Validation, and Dependency Metadata  
**Source File:** `crates/cli/src/package.rs` (`Lockfile`, `LockfileSchema`, `LockPackageEntry`, `LockRootPackage`)  
**Audited Revision:** `826b16de4e20fc2bcb4fdea4f2257bff88ed4ea4`  
**Auditor:** Principal Compiler Security Engineer & Supply-Chain Security Architect  
**Status:** **ACCEPTED WITH LIMITATIONS** (Technical Audit Passed, 1 Low-Severity Residual Finding)

---

## 1. Executive Summary & Audit Context

Earlier peer reviews correctly rejected the initial Phase 1 lockfile implementation because it relied on a custom line-by-line state-machine parser. Even after improvements, the custom parser suffered from structural limitations: it could not handle all TOML escape sequences, multiline strings, duplicate table declarations, or arbitrary TOML data types, and it permitted unknown root-level and table-level keys.

In commit `826b16d`, the custom parser was completely replaced with the industry-standard `toml` crate (v1.2.2+spec-1.1.0) and `serde` (v1.0.229) with typed deserialization and compile-time `#[serde(deny_unknown_fields)]` annotations.

This document reviews:
1. The standard TOML deserialization engine and strict schema enforcement.
2. Semantic validation for Git and registry dependencies.
3. Canonical lockfile serialization.
4. Concurrency, file replacement atomicity, and residual risks (FIND-PH1-AUDIT-02).

---

## 2. Standard TOML Deserialization & Schema Hardening

The lockfile data structures in `crates/cli/src/package.rs:445-480` enforce strict typing and schema boundaries:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockPackageEntry {
    pub name: String,
    pub version: String,
    pub source: String,
    #[serde(default)]
    pub commit: Option<String>,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockRootPackage {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockfileSchema {
    pub version: u32,
    pub package: LockRootPackage,
    #[serde(default)]
    pub packages: Vec<LockPackageEntry>,
}
```

### Security Properties Verified:
1. **Full TOML 1.1 Specification Compliance:** Handled directly by `toml::from_str`. All valid TOML formatting—including comments, multiline strings, unicode escapes (`\uXXXX`, `\UXXXXXXXX`), whitespace variations, and array representations—is parsed correctly.
2. **Rejection of Unknown Fields:** `#[serde(deny_unknown_fields)]` rejects any unrecognized keys at the root, package, or dependency levels. Attackers cannot inject unmonitored metadata fields into lockfiles.
3. **Rejection of Duplicate Keys & Tables:** Handled natively by `toml::from_str`. Duplicate keys within a table or duplicate `[package]` sections trigger immediate parse errors (`"duplicate key"`, `"duplicate table"`).
4. **Strict Schema Versioning:** `schema.version` must strictly equal `1`. Any unsupported version returns:
   `"Unsupported lockfile schema version '<v>'; expected version = 1"`

---

## 3. Semantic Validation of Dependencies

Beyond syntactic TOML parsing, `Lockfile::parse` (`crates/cli/src/package.rs:483-540`) executes strict semantic verification over the deserialized entries:

### 3.1 Duplicate Package Name Rejection
A `HashSet<String>` tracks every package name in `[[packages]]`. If any package name appears more than once, parsing aborts immediately:
`"Duplicate package entry in lockfile: '<name>'"`

### 3.2 SHA-256 Checksum Validation
The checksum field must conform to cryptographic SHA-256 specifications:
- Accepts either raw 64-character hex strings or strings prefixed with `sha256:`.
- Enforces exact length: `clean_csum.len() == 64`.
- Enforces character set: all characters must be ASCII hexadecimal digits (`0-9`, `a-f`, `A-F`).
- Normalizes to lowercase hex for canonical comparison.
- Invalid checksums return:
  `"Malformed SHA-256 checksum '<csum>' for package '<name>' (must be 64 hexadecimal characters)"`

### 3.3 Git Dependency Commit Enforcement
For dependencies with a source URL starting with `git+`:
- The `commit` field is **mandatory**. Missing commit hashes fail with:
  `"Missing 'commit' for git dependency '<name>'"`
- The commit hash must be an exact 40-character hexadecimal string representing an immutable Git commit object ID:
  `"Invalid git commit hash '<hash>' for package '<name>' (must be 40 hexadecimal characters)"`
- This ensures Git dependencies cannot float to unpinned branch heads (`main`, `HEAD`), preventing branch-sliding supply chain attacks.

---

## 4. Deterministic Canonical Serialization

When writing lockfiles, Syntra uses `Lockfile::to_canonical_string()` (`crates/cli/src/package.rs:542-566`):
- Dependencies in `packages` are sorted lexicographically by name (`pkgs.sort_by(|a, b| a.name.cmp(&b.name))`).
- Emits standardized headers and indentation.
- Generates reproducible, diff-friendly lockfiles across different platforms and runs.

---

## 5. Lockfile Write Mechanics & Atomicity Analysis

### FIND-PH1-AUDIT-02 (Severity: LOW):
**Finding Title:** Non-Atomic Lockfile File Replacement and Lack of Advisory File Locking.

### Vulnerability Analysis:
In `crates/cli/src/package.rs:997-1003` and `crates/cli/src/lib.rs`:
```rust
if let Err(e) = fs::write(&lock_path, &lock_content) {
    eprintln!("❌ Failed to write {primary_lock}: {e}");
    exit(1);
}
if is_syn {
    let _ = fs::write(manifest_dir.join("syntra.lock"), &lock_content);
}
```

1. **Non-Atomic File In-Place Replacement:**
   `fs::write` opens the target file with `O_TRUNC | O_WRONLY | O_CREAT`. If the process is terminated (e.g., SIGINT, power loss, OOM killer) mid-write, `Package.lock` is left truncated or half-written. The next `syn build` will fail with a corrupt lockfile.
2. **Race Window in Concurrent Tool Invocations:**
   If multiple processes (e.g., an IDE language server running `syn check` alongside a terminal `syn install` or `syn lock`) execute concurrently, there is no advisory file lock (`flock` on Unix or `LockFileEx` on Windows) guarding `Package.lock`. Two concurrent writers can interleave writes, leading to file corruption.

### Remediation Recommendation:
Implement an atomic write-rename pattern:
1. Write the new lockfile content to a temporary file in the same directory (e.g., `Package.lock.tmp.<pid>`).
2. Sync file contents to disk (`file.sync_all()`).
3. Atomically rename the temporary file over the target (`fs::rename(tmp_path, lock_path)`), which is atomic on all POSIX and modern Windows systems.
4. Guard lockfile updates with an advisory file lock (`fs2::FileExt::lock_exclusive` or standard OS file locks).

---

## 6. Verification Evidence

The lockfile deserialization and validation subsystem is covered by 7 dedicated adversarial tests in `crates/cli/tests/test_package_integrity.rs`:
- `test_lockfile_parse_valid_format`: Verified clean deserialization of valid TOML lockfiles.
- `test_lockfile_rejects_duplicate_packages`: Verified rejection of duplicate `[[packages]]` entries.
- `test_lockfile_rejects_malformed_checksum`: Verified rejection of 63-char or non-hex checksums.
- `test_git_dependency_missing_commit_detected`: Verified rejection of Git dependencies without commits.
- `test_git_dependency_invalid_commit_hash_detected`: Verified rejection of invalid commit hashes.
- `test_lockfile_rejects_duplicate_keys_standard_toml`: Verified standard TOML duplicate key rejection.
- `test_lockfile_rejects_duplicate_table_standard_toml`: Verified standard TOML duplicate table rejection.

**Audit Assessment:** The lockfile parsing engine is fully compliant with standard TOML, strictly typed, and resistant to malicious tampering. FIND-PH1-AUDIT-02 is recorded in the improvement register for future atomic I/O enhancement.
