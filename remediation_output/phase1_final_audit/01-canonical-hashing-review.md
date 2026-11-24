# Syntra Phase 1 Final Security Acceptance Audit
# Document 01: Canonical Hashing Implementation Review

**Target Component:** Package Content Cryptographic Hashing  
**Source File:** `crates/cli/src/package.rs` (`hash_directory_tree_sha256`, `collect_canonical_entries`, `should_exclude`, `validate_and_normalize_symlink_target`)  
**Audited Revision:** `826b16de4e20fc2bcb4fdea4f2257bff88ed4ea4`  
**Auditor:** Principal Compiler Security Engineer & Supply-Chain Security Architect  
**Status:** **ACCEPTED WITH LIMITATIONS** (Technical Audit Passed, 1 Low-Severity Residual Finding)

---

## 1. Executive Summary & Context

The initial baseline finding (**FIND-001**) identified that package lockfile generation in Syntra relied on a non-cryptographic 64-bit DJB2 hash computed exclusively over package name strings. That implementation completely ignored file contents, Git commit trees, directory structures, and filesystem permissions.

Subsequent peer review identified that earlier remediation attempts introduced:
1. Broad file extension exclusions (e.g., omitting `.so`, `.dylib`, `.dll`, `.exe`, `.a`, `.o`, and nested `.lock` files).
2. Fail-open hash fallbacks (falling back to URL or name hashing when directory read operations failed).
3. Ambiguous serialization formats.

In commit `826b16d`, the canonical hashing subsystem was hardened into a deterministic, fail-closed FIPS 180-4 SHA-256 tree hashing architecture. This audit evaluates whether the canonical representation is collision-resistant, deterministic across supported operating systems, and strictly bounded against filesystem manipulation attacks.

---

## 2. Evaluation of `hash_directory_tree_sha256` Architecture

The core hashing logic is implemented in `crates/cli/src/package.rs:262-312`.

```rust
pub fn hash_directory_tree_sha256(dir: &Path) -> Result<String, String>
```

### Architectural Guarantees:
- **Single-File Handling:** If the target path is an individual file, it computes standard SHA-256 over raw file bytes.
- **Fail-Closed Pre-conditions:** Returns `Err` if the directory does not exist or if permissions prevent reading directory metadata.
- **Lexicographical Ordering:** Traverses all child elements via `collect_canonical_entries`, normalizes relative paths to forward slashes, and performs strict lexicographical sorting (`entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path))`) before any hasher ingestion.
- **Deterministic Hash Ingestion:** Ingests framed entry records into a single `Sha256` context.

---

## 3. Canonical Entry Serialization & Framing

For each canonical entry in the sorted list, the hasher receives a distinct, framed sequence:

```rust
hasher.update(b"ENTRY\n");
hasher.update(b"path:");
hasher.update(entry.rel_path.as_bytes());
hasher.update(b"\n");

hasher.update(b"mode:");
if entry.is_executable {
    hasher.update(b"0755\n");
} else {
    hasher.update(b"0644\n");
}

if entry.is_symlink {
    hasher.update(b"type:symlink\n");
    let target = entry.symlink_target.as_deref().unwrap_or("");
    hasher.update(b"target:");
    hasher.update(target.as_bytes());
    hasher.update(b"\n");
} else {
    hasher.update(b"type:file\n");
    hasher.update(b"size:");
    hasher.update(entry.content.len().to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(&entry.content);
    hasher.update(b"\n");
}

hasher.update(b"END_ENTRY\n");
```

### Cryptographic Framing Analysis:
1. **Length Prefixing & Explicit Delimiters:** Ingestion of file contents is preceded by an explicit length header (`size:<length>\n`). This eliminates concatenation ambiguity attacks where the boundary between a file's content and subsequent entry headers might otherwise be blurred.
2. **Permission Distinctions:** POSIX executable permissions (`0o111`) are explicitly mapped to canonical strings (`0755` vs `0644`). A simple `chmod +x` or `chmod -x` on any source or binary file alters the calculated digest.
3. **Entry Framing Envelope:** The `ENTRY\n` and `END_ENTRY\n` tokens enforce clear record boundaries.

---

## 4. Path Normalization & Determinism

Path normalization is handled in `crates/cli/src/package.rs:172-198`:

1. `path.strip_prefix(root_dir)` strips the local installation prefix, ensuring that packages installed at `/home/user/.syntra/packages/...` and `/opt/syntra/...` yield identical relative paths.
2. Every path component is inspected using `std::path::Component`:
   - `Normal(c)`: Extracted as UTF-8; non-UTF-8 components return an explicit error.
   - `ParentDir` (`..`): Explicitly rejected with `"Path traversal component '..' found in relative path"`.
   - `CurDir` (`.`): Skipped.
3. Normalized relative components are joined exclusively with forward slashes (`/`), guaranteeing cross-platform byte uniformity between Windows (`\`) and POSIX (`/`) filesystems.

---

## 5. Symlink Containment & Traversal Defense

Symlinks represent a prime vector for supply-chain attacks, allowing malicious packages to exfiltrate host files (`/etc/passwd`, `~/.ssh/id_rsa`) or escape package directories.

The implementation handles symlinks through `validate_and_normalize_symlink_target` (`crates/cli/src/package.rs:84-144`):

```rust
pub fn validate_and_normalize_symlink_target(
    root_dir: &Path,
    symlink_path: &Path,
    target: &Path,
) -> Result<String, String>
```

### Security Defenses Verified:
1. **Rejection of Absolute Symlinks:** Targets starting with `/` or Windows drive prefixes are rejected immediately (`target.is_absolute() -> Err`).
2. **Lexical Stack Traversal Tracking:** The function models a virtual path stack relative to `root_dir`. Traversal components (`..`) pop from the stack. If the stack is empty when encountering `..`, the symlink is proven to escape the package root and is rejected with:
   `"Path traversal attempt: symlink '<path>' escapes package root with target '<target>'"`
3. **Symlink Metadata vs Content:** In `collect_canonical_entries`, symlinks are audited using `fs::symlink_metadata` rather than `fs::metadata`. The canonical tree records the target path string rather than reading through the link, preventing circular reference lockups or host data ingestion.

---

## 6. Precompiled Binaries & Narrow Exclusion Boundary

Earlier iterations unsafely excluded precompiled native binaries and shared objects. Commit `826b16d` replaced broad exclusions with narrow, root-relative rules in `should_exclude` (`crates/cli/src/package.rs:54-82`):

### Narrowed Exclusions:
1. **Root-relative build and toolchain directories:** `.git`, `.syntra`, `target` (and their subtrees).
2. **OS Artifacts:** Exact filenames `.DS_Store`, `Thumbs.db`, and files starting with `._`.
3. **Editor Swap Files:** Files ending with `.swp` or `~`.
4. **Standalone Package Archives:** Root-level `.synpkg` archives.

### Critical Inclusions:
- Precompiled libraries (`.so`, `.dylib`, `.dll`, `.a`) anywhere in the package tree are **strictly hashed**.
- Executable binaries (`.exe`, ELF, Mach-O) are **strictly hashed**.
- Nested `.lock` files inside dependency directories are **strictly hashed**.
- Native object files (`.o`, `.obj`) are **strictly hashed**.

Adversarial test `test_nested_exclusions_and_precompiled_binaries_included` proves that tampering with a precompiled shared library changes the tree hash from `a71317...` to `55d5bc...`.

---

## 7. Fail-Closed Error Handling vs Historical Fail-Open Fallbacks

Historical iterations fell back to hashing dependency names or repository URLs if reading a directory failed.

In the current implementation:
- Directory traversal failures in `fs::read_dir` return `Result::Err`.
- Entry read failures in `fs::read` return `Result::Err`.
- Symlink resolution failures in `fs::read_link` return `Result::Err`.
- Special filesystem objects (FIFOs, Unix domain sockets, block/character devices) return `Result::Err` with `"Unsupported filesystem object '...' (special device, socket, or fifo) in package tree"`.

Zero fail-open fallbacks exist in the tree hashing path.

---

## 8. Residual Findings & Edge Cases

### FIND-PH1-AUDIT-04 (Severity: LOW): Path Delimiter Framing
While `entry.content` is length-prefixed (`size:<len>\n`), `entry.rel_path` is ingested with a single newline delimiter:
```rust
hasher.update(b"path:");
hasher.update(entry.rel_path.as_bytes());
hasher.update(b"\n");
```
On POSIX filesystems, filenames can theoretically contain literal newline characters (`\n`). If an attacker crafted a file whose name contained embedded newlines matching entry header grammar (`\nmode:0755\ntype:...`), this could theoretically create a collision opportunity between two crafted filenames.

- **Mitigation / Recommendation:** Disallow newline characters in relative file paths during normalization (`if rel_path_str.contains('\n') { return Err(...) }`) or length-prefix the relative path string in the canonical entry format.
- **Risk Level:** Low. Exploitability requires creating arbitrary filenames with raw newlines inside a package, which is generally rejected by Git and archive unpackers.

---

## 9. Verification Evidence

The hashing implementation was evaluated against 7 specific adversarial test scenarios in `crates/cli/tests/test_package_integrity.rs`:
- `test_hash_directory_tree_sha256_determinism`: Passed.
- `test_hash_directory_tree_sha256_detects_content_modification`: Passed.
- `test_hash_directory_tree_sha256_detects_filename_rename`: Passed.
- `test_hash_directory_tree_sha256_detects_executable_permission_change`: Passed.
- `test_hash_directory_tree_sha256_symlink_inside_package`: Passed.
- `test_hash_directory_tree_sha256_symlink_escaping_package_fails`: Passed.
- `test_nested_exclusions_and_precompiled_binaries_included`: Passed.

**Audit Assessment:** The canonical directory tree hashing implementation satisfies all core requirements for supply-chain integrity, with 1 low-severity framing issue noted in the improvement register.
