use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use syntra_cli::package::{
    generate_lock_content, hash_bytes_sha256, hash_directory_tree_sha256,
    verify_locked_dependencies, Lockfile,
};
use syntra_frontend::{PackageDependency, PackageManifest};

#[test]
fn test_sha256_known_vectors() {
    // NIST Standard vector for empty input
    assert_eq!(
        hash_bytes_sha256(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    // Standard vector for "hello world"
    assert_eq!(
        hash_bytes_sha256(b"hello world"),
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn test_directory_tree_hash_determinism() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_tree_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("src")).unwrap();

    fs::write(tmp.join("Package.syn"), "package Foo { name = \"foo\" }").unwrap();
    fs::write(
        tmp.join("src").join("main.syn"),
        "public func main(): int { return 0; }",
    )
    .unwrap();
    fs::write(
        tmp.join("src").join("utils.syn"),
        "public func helper(): void {}",
    )
    .unwrap();

    let hash1 = hash_directory_tree_sha256(&tmp).expect("Tree hash calculation failed");
    let hash2 = hash_directory_tree_sha256(&tmp).expect("Tree hash calculation failed");

    assert_eq!(hash1, hash2, "Tree hash must be completely deterministic");
    assert_eq!(
        hash1.len(),
        64,
        "SHA-256 digest must be 64 hexadecimal characters"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_tampering_file_content_detected() {
    let tmp =
        std::env::temp_dir().join(format!("syntra_test_tamper_content_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("math_lib");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    fs::write(
        dep_dir.join("src").join("lib.syn"),
        "public func add(int a, int b): int { return a + b; }",
    )
    .unwrap();

    let manifest = PackageManifest {
        name: "test_app".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "math_lib".to_string(),
            version: Some("1.0.0".to_string()),
            path: Some("deps/math_lib".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    // 1. Generate canonical lockfile
    let (lock_content, count) =
        generate_lock_content(&tmp, &manifest).expect("Lock generation failed");
    assert_eq!(count, 1);
    assert!(lock_content.contains("name = \"math_lib\""));
    assert!(lock_content.contains("checksum = \""));

    // 2. Verify untampered passes
    let verify_clean = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_clean.is_ok(),
        "Untampered dependency must verify cleanly"
    );

    // 3. Tamper with file content by modifying 1 byte
    fs::write(
        dep_dir.join("src").join("lib.syn"),
        "public func add(int a, int b): int { return a - b; }",
    )
    .unwrap();

    // 4. Verify tampering is detected
    let verify_tampered = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_tampered.is_err(),
        "Tampered file content MUST be rejected"
    );
    let errs = verify_tampered.unwrap_err();
    assert!(errs
        .iter()
        .any(|e| e.contains("Tampering or modification detected for dependency 'math_lib'")));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_tampering_add_or_delete_file_detected() {
    let tmp =
        std::env::temp_dir().join(format!("syntra_test_tamper_add_del_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("crypto_lib");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    fs::write(dep_dir.join("src").join("sha.syn"), "func sha(): void {}").unwrap();

    let manifest = PackageManifest {
        name: "test_crypto".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "crypto_lib".to_string(),
            version: Some("0.5.0".to_string()),
            path: Some("deps/crypto_lib".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (lock_content, _) = generate_lock_content(&tmp, &manifest).expect("Lock generation failed");
    assert!(verify_locked_dependencies(&tmp, &manifest, &lock_content).is_ok());

    // Inject unauthorized extra file
    fs::write(
        dep_dir.join("src").join("backdoor.syn"),
        "func backdoor(): void {}",
    )
    .unwrap();
    let verify_injected = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_injected.is_err(),
        "Injected unauthorized file MUST be rejected"
    );

    // Delete original file
    fs::remove_file(dep_dir.join("src").join("sha.syn")).unwrap();
    let verify_deleted = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_deleted.is_err(),
        "Deleted file in dependency tree MUST be rejected"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_tampering_rename_file_detected() {
    let tmp =
        std::env::temp_dir().join(format!("syntra_test_tamper_rename_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("logger");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    fs::write(
        dep_dir.join("src").join("log.syn"),
        "func log(string s): void {}",
    )
    .unwrap();

    let manifest = PackageManifest {
        name: "test_rename".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "logger".to_string(),
            version: Some("0.1.0".to_string()),
            path: Some("deps/logger".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (lock_content, _) = generate_lock_content(&tmp, &manifest).expect("Lock generation failed");
    assert!(verify_locked_dependencies(&tmp, &manifest, &lock_content).is_ok());

    // Rename log.syn to logger.syn with identical content
    fs::rename(
        dep_dir.join("src").join("log.syn"),
        dep_dir.join("src").join("logger.syn"),
    )
    .unwrap();

    let verify_renamed = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_renamed.is_err(),
        "Renaming a file in dependency tree MUST be detected"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
#[cfg(unix)]
fn test_tampering_executable_permission_detected() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_tamper_perms_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("tool_pkg");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    let script = dep_dir.join("src").join("run.sh");
    fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();

    // Set non-executable mode (0644)
    fs::set_permissions(&script, fs::Permissions::from_mode(0o644)).unwrap();

    let manifest = PackageManifest {
        name: "test_perms".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "tool_pkg".to_string(),
            version: Some("0.1.0".to_string()),
            path: Some("deps/tool_pkg".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (lock_content, _) = generate_lock_content(&tmp, &manifest).expect("Lock generation failed");
    assert!(verify_locked_dependencies(&tmp, &manifest, &lock_content).is_ok());

    // Alter executable permission bit: set 0755
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let verify_perm_tampered = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_perm_tampered.is_err(),
        "Changing executable permissions MUST alter tree hash and be rejected"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
#[cfg(unix)]
fn test_symlink_target_tampering_and_valid_handling() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_symlink_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("sym_lib");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    fs::write(
        dep_dir.join("src").join("target1.syn"),
        "func t1(): void {}",
    )
    .unwrap();
    fs::write(
        dep_dir.join("src").join("target2.syn"),
        "func t2(): void {}",
    )
    .unwrap();

    // Create relative symlink: alias.syn -> target1.syn
    std::os::unix::fs::symlink("target1.syn", dep_dir.join("src").join("alias.syn")).unwrap();

    let manifest = PackageManifest {
        name: "test_symlink".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "sym_lib".to_string(),
            version: Some("1.0.0".to_string()),
            path: Some("deps/sym_lib".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (lock_content, _) = generate_lock_content(&tmp, &manifest).expect("Lock generation failed");
    assert!(verify_locked_dependencies(&tmp, &manifest, &lock_content).is_ok());

    // Retarget symlink: alias.syn -> target2.syn
    fs::remove_file(dep_dir.join("src").join("alias.syn")).unwrap();
    std::os::unix::fs::symlink("target2.syn", dep_dir.join("src").join("alias.syn")).unwrap();

    let verify_retargeted = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_retargeted.is_err(),
        "Changing symlink target MUST alter tree hash and be rejected"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
#[cfg(unix)]
fn test_path_traversal_symlink_rejected() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_traversal_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("evil_lib");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    fs::write(dep_dir.join("src").join("main.syn"), "func main(): void {}").unwrap();

    // Symlink that attempts to escape package root
    std::os::unix::fs::symlink("../../outside.txt", dep_dir.join("src").join("escape.syn"))
        .unwrap();

    let hash_res = hash_directory_tree_sha256(&dep_dir);
    assert!(
        hash_res.is_err(),
        "Symlink pointing outside package root MUST be rejected with traversal error"
    );
    let err = hash_res.unwrap_err();
    assert!(
        err.contains("Path traversal attempt") || err.contains("escapes package root"),
        "Error message must specify path traversal attempt: {err}"
    );

    // Absolute symlink must also be rejected
    fs::remove_file(dep_dir.join("src").join("escape.syn")).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", dep_dir.join("src").join("abs_escape.syn")).unwrap();

    let abs_hash_res = hash_directory_tree_sha256(&dep_dir);
    assert!(
        abs_hash_res.is_err(),
        "Absolute symlink MUST be rejected with traversal error"
    );
    assert!(abs_hash_res
        .unwrap_err()
        .contains("Absolute symlink rejected"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_missing_package_directory_fails_explicitly() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_missing_dir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let manifest = PackageManifest {
        name: "test_missing".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "non_existent_dep".to_string(),
            version: Some("1.0.0".to_string()),
            path: Some("deps/missing".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Must NOT fall back to URL or name hashing; MUST return explicit Err
    let lock_res = generate_lock_content(&tmp, &manifest);
    assert!(
        lock_res.is_err(),
        "Missing dependency directory MUST fail lock generation, never fall back"
    );
    let err = lock_res.unwrap_err();
    assert!(err.contains("does not exist at"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_lockfile_schema_validation_rejects_malformed_syntax() {
    let malformed_toml = r#"
version = 1
[package]
name = "demo"
version = "0.1.0"

[[packages]]
name = "dep"
version = "1.0.0"
source = "path:deps/dep"
checksum = "not_a_hex_checksum"
"#;

    let res = Lockfile::parse(malformed_toml);
    assert!(
        res.is_err(),
        "Lockfile with malformed checksum MUST fail schema validation"
    );
    assert!(res.unwrap_err().contains("Malformed SHA-256 checksum"));
}

#[test]
fn test_lockfile_schema_validation_rejects_duplicate_entries() {
    let duplicate_toml = r#"
version = 1
[package]
name = "demo"
version = "0.1.0"

[[packages]]
name = "dup_lib"
version = "1.0.0"
source = "path:deps/dup"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

[[packages]]
name = "dup_lib"
version = "1.0.0"
source = "path:deps/dup"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"#;

    let res = Lockfile::parse(duplicate_toml);
    assert!(
        res.is_err(),
        "Lockfile with duplicate package entries MUST be rejected"
    );
    assert!(res.unwrap_err().contains("Duplicate package entry"));
}

#[test]
fn test_lockfile_schema_validation_rejects_unsupported_version() {
    let invalid_version_toml = r#"
version = 99
[package]
name = "demo"
version = "0.1.0"
"#;

    let res = Lockfile::parse(invalid_version_toml);
    assert!(
        res.is_err(),
        "Unsupported lockfile schema version MUST be rejected"
    );
    assert!(res
        .unwrap_err()
        .contains("Unsupported lockfile schema version"));
}

#[test]
fn test_lockfile_missing_and_unexpected_dependencies_detected() {
    let tmp =
        std::env::temp_dir().join(format!("syntra_test_missing_unexp_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_a = tmp.join("deps").join("dep_a");
    fs::create_dir_all(dep_a.join("src")).unwrap();
    fs::write(dep_a.join("src").join("a.syn"), "func a(): void {}").unwrap();

    let manifest = PackageManifest {
        name: "test_mismatch".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![
            PackageDependency {
                name: "dep_a".to_string(),
                version: Some("1.0.0".to_string()),
                path: Some("deps/dep_a".to_string()),
                ..Default::default()
            },
            PackageDependency {
                name: "dep_b".to_string(),
                version: Some("1.0.0".to_string()),
                path: Some("deps/dep_b".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    // Lockfile only contains dep_a and an unmanifested dep_c
    let lock_content = r#"
version = 1
[package]
name = "test_mismatch"
version = "0.1.0"

[[packages]]
name = "dep_a"
version = "1.0.0"
source = "path:deps/dep_a"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

[[packages]]
name = "dep_c"
version = "1.0.0"
source = "path:deps/dep_c"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"#;

    let verify_res = verify_locked_dependencies(&tmp, &manifest, lock_content);
    assert!(verify_res.is_err());
    let errs = verify_res.unwrap_err();

    assert!(errs
        .iter()
        .any(|e| e.contains("missing from lockfile") && e.contains("dep_b")));
    assert!(errs
        .iter()
        .any(|e| e.contains("unexpected dependency 'dep_c'")));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_git_dependency_missing_commit_detected() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_git_commit_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let lock_content = r#"
version = 1
[package]
name = "test_git"
version = "0.1.0"

[[packages]]
name = "remote_git_lib"
version = "1.0.0"
source = "git+https://github.com/example/lib.git#main"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"#;

    let res = Lockfile::parse(lock_content);
    assert!(
        res.is_err(),
        "Git dependency in lockfile without commit hash MUST be rejected"
    );
    assert!(res
        .unwrap_err()
        .contains("Missing 'commit' for git dependency"));
}

#[test]
#[cfg(unix)]
fn test_unreadable_file_fails_explicitly() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_unreadable_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("locked_pkg");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    let secret_file = dep_dir.join("src").join("secret.syn");
    fs::write(&secret_file, "func secret(): void {}").unwrap();

    // Remove all read permissions (0000)
    fs::set_permissions(&secret_file, fs::Permissions::from_mode(0o000)).unwrap();

    let res = hash_directory_tree_sha256(&dep_dir);
    // Restore permissions so cleanup works
    let _ = fs::set_permissions(&secret_file, fs::Permissions::from_mode(0o644));

    assert!(
        res.is_err(),
        "Unreadable file in dependency tree MUST return explicit error, never be silently skipped"
    );
    let err = res.unwrap_err();
    assert!(err.contains("Failed to read file") || err.contains("Permission denied"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
#[cfg(unix)]
fn test_unsupported_fifo_fails_explicitly() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_fifo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("fifo_pkg");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    let fifo_path = dep_dir.join("src").join("my_fifo");

    // Create a FIFO via system mkfifo
    let status = std::process::Command::new("mkfifo")
        .arg(&fifo_path)
        .status();

    if let Ok(st) = status {
        if st.success() {
            let res = hash_directory_tree_sha256(&dep_dir);
            assert!(
                res.is_err(),
                "FIFO in package tree MUST be rejected as unsupported filesystem object"
            );
            let err = res.unwrap_err();
            assert!(err.contains("Unsupported filesystem object"));
        }
    }

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_git_dependency_altered_commit_detected() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_git_tamper_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    // 1. Initialize a real local git repo as our dependency
    let git_repo = tmp.join("git_dep");
    fs::create_dir_all(git_repo.join("src")).unwrap();
    fs::write(
        git_repo.join("src").join("lib.syn"),
        "func git_lib(): void {}",
    )
    .unwrap();

    let _ = std::process::Command::new("git")
        .current_dir(&git_repo)
        .args(["init", "--quiet"])
        .status();
    let _ = std::process::Command::new("git")
        .current_dir(&git_repo)
        .args(["config", "user.email", "test@syntra.lang"])
        .status();
    let _ = std::process::Command::new("git")
        .current_dir(&git_repo)
        .args(["config", "user.name", "Syntra Test"])
        .status();
    let _ = std::process::Command::new("git")
        .current_dir(&git_repo)
        .args(["config", "commit.gpgsign", "false"])
        .status();
    let _ = std::process::Command::new("git")
        .current_dir(&git_repo)
        .args(["add", "."])
        .status();
    let mut commit_cmd = std::process::Command::new("git");
    commit_cmd
        .current_dir(&git_repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args([
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "Initial commit",
            "--quiet",
            "--no-gpg-sign",
        ]);
    let commit_status = commit_cmd
        .status()
        .expect("git commit command failed to execute");
    assert!(commit_status.success(), "Initial git commit must succeed");

    let commit1 = std::process::Command::new("git")
        .current_dir(&git_repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap();

    let tree_hash = hash_directory_tree_sha256(&git_repo).unwrap();

    // 2. Lockfile records commit1
    let manifest = PackageManifest {
        name: "app".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "git_dep".to_string(),
            version: Some("1.0.0".to_string()),
            git: Some(git_repo.to_string_lossy().to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let lock_content = format!(
        r#"
version = 1
[package]
name = "app"
version = "0.1.0"

[[packages]]
name = "git_dep"
version = "1.0.0"
source = "git+{url}"
commit = "{commit1}"
checksum = "{tree_hash}"
"#,
        url = git_repo.to_string_lossy(),
        commit1 = commit1,
        tree_hash = tree_hash
    );

    // 3. Fake a second commit by tampering the commit recorded in lockfile
    let tampered_lock = lock_content.replace(&commit1, "0000000000000000000000000000000000000000");

    let verify_tampered = verify_locked_dependencies(&tmp, &manifest, &tampered_lock);
    assert!(
        verify_tampered.is_err(),
        "Altered Git revision commit MUST be detected and rejected"
    );
    let errs = verify_tampered.unwrap_err();
    assert!(errs.iter().any(|e| e.contains("Git revision mismatch")));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_tampering_native_library_and_lockfile_detected() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_native_libs_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("c_interop");
    fs::create_dir_all(dep_dir.join("lib")).unwrap();

    // Include native dynamic library, static library, and lockfile
    let dylib_path = dep_dir.join("lib").join("libnative.dylib");
    let static_path = dep_dir.join("lib").join("libstatic.a");
    let internal_lock = dep_dir.join("Cargo.lock");

    fs::write(&dylib_path, [0xCA, 0xFE, 0xBA, 0xBE]).unwrap();
    fs::write(&static_path, [0x21, 0x3C, 0x61, 0x72]).unwrap();
    fs::write(&internal_lock, "version = 1\n").unwrap();

    let manifest = PackageManifest {
        name: "test_native".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "c_interop".to_string(),
            version: Some("1.0.0".to_string()),
            path: Some("deps/c_interop".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (lock_content, _) = generate_lock_content(&tmp, &manifest).expect("Lock generation failed");
    assert!(verify_locked_dependencies(&tmp, &manifest, &lock_content).is_ok());

    // 1. Tamper with native .dylib
    fs::write(&dylib_path, [0xDE, 0xAD, 0xBE, 0xEF]).unwrap();
    let verify_dylib = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_dylib.is_err(),
        "Altering native shared library in dependency MUST alter checksum and be rejected"
    );

    // Restore .dylib and tamper with internal .lock file
    fs::write(&dylib_path, [0xCA, 0xFE, 0xBA, 0xBE]).unwrap();
    fs::write(&internal_lock, "version = 2\n").unwrap();
    let verify_lock = verify_locked_dependencies(&tmp, &manifest, &lock_content);
    assert!(
        verify_lock.is_err(),
        "Altering lockfile in dependency MUST alter checksum and be rejected"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_lockfile_parser_rejects_duplicate_keys_in_package_entry() {
    let duplicate_key_toml = r#"
version = 1
[package]
name = "test_dup_key"
version = "0.1.0"

[[packages]]
name = "foo"
name = "bar"
version = "1.0.0"
source = "path:deps/foo"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"#;

    let res = Lockfile::parse(duplicate_key_toml);
    assert!(
        res.is_err(),
        "Duplicate key within [[packages]] block MUST be rejected"
    );
    assert!(res
        .unwrap_err()
        .contains("Duplicate key 'name' in [[packages]] block"));
}

#[test]
fn test_lockfile_parser_rejects_duplicate_root_keys_and_sections() {
    let duplicate_section_toml = r#"
version = 1
[package]
name = "test_dup_sec"
version = "0.1.0"

[package]
name = "test_dup_sec_2"
version = "0.1.0"
"#;

    let res = Lockfile::parse(duplicate_section_toml);
    assert!(res.is_err(), "Duplicate [package] section MUST be rejected");
    assert!(res.unwrap_err().contains("Duplicate [package] section"));

    let duplicate_root_key_toml = r#"
version = 1
version = 1
[package]
name = "test_dup_sec"
version = "0.1.0"
"#;

    let res_key = Lockfile::parse(duplicate_root_key_toml);
    assert!(res_key.is_err(), "Duplicate top-level key MUST be rejected");
    assert!(res_key
        .unwrap_err()
        .contains("Duplicate top-level key 'version'"));
}

#[test]
fn test_lockfile_parser_unescapes_toml_string_literals() {
    let toml_with_escapes = r#"
version = 1
[package]
name = "test\"app"
version = "0.1.0\tbuild"

[[packages]]
name = "lib\u0041"
version = "1.0.0"
source = "path:deps/lib\\foo"
checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"#;

    let lockfile = Lockfile::parse(toml_with_escapes).expect("Failed to parse valid TOML escapes");
    assert_eq!(lockfile.package.name, "test\"app");
    assert_eq!(lockfile.package.version, "0.1.0\tbuild");
    assert_eq!(lockfile.packages[0].name, "libA");
    assert_eq!(lockfile.packages[0].source, "path:deps/lib\\foo");

    // Invalid escape sequence must be rejected
    let invalid_escape = r#"
version = 1
[package]
name = "test\qapp"
version = "0.1.0"
"#;
    let res_invalid = Lockfile::parse(invalid_escape);
    assert!(
        res_invalid.is_err(),
        "Invalid TOML escape sequence MUST be rejected"
    );
    assert!(res_invalid
        .unwrap_err()
        .contains("Invalid TOML escape sequence"));
}

#[test]
fn test_lockfile_root_package_identity_mismatch_detected() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_root_id_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let manifest = PackageManifest {
        name: "actual_app".to_string(),
        version: "1.0.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![],
        ..Default::default()
    };

    let lock_wrong_name = r#"
version = 1
[package]
name = "forged_app"
version = "1.0.0"
"#;

    let res_name = verify_locked_dependencies(&tmp, &manifest, lock_wrong_name);
    assert!(
        res_name.is_err(),
        "Root package name mismatch MUST be detected"
    );
    assert!(res_name
        .unwrap_err()
        .iter()
        .any(|e| e.contains("root package name mismatch")));

    let lock_wrong_version = r#"
version = 1
[package]
name = "actual_app"
version = "2.0.0"
"#;

    let res_ver = verify_locked_dependencies(&tmp, &manifest, lock_wrong_version);
    assert!(
        res_ver.is_err(),
        "Root package version mismatch MUST be detected"
    );
    assert!(res_ver
        .unwrap_err()
        .iter()
        .any(|e| e.contains("root package version mismatch")));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_dependency_version_and_source_mismatch_detected() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_dep_id_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let dep_dir = tmp.join("deps").join("my_lib");
    fs::create_dir_all(dep_dir.join("src")).unwrap();
    fs::write(
        dep_dir.join("src").join("lib.syn"),
        "func my_lib(): void {}",
    )
    .unwrap();

    let manifest = PackageManifest {
        name: "app".to_string(),
        version: "0.1.0".to_string(),
        edition: "2026".to_string(),
        dependencies: vec![PackageDependency {
            name: "my_lib".to_string(),
            version: Some("1.0.0".to_string()),
            path: Some("deps/my_lib".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let (lock_content, _) = generate_lock_content(&tmp, &manifest).expect("Lock generation failed");
    assert!(verify_locked_dependencies(&tmp, &manifest, &lock_content).is_ok());

    // 1. Mismatch in version
    let lock_tampered_ver = lock_content.replace("version = \"1.0.0\"", "version = \"2.0.0\"");
    let res_ver = verify_locked_dependencies(&tmp, &manifest, &lock_tampered_ver);
    assert!(
        res_ver.is_err(),
        "Dependency version mismatch MUST be detected"
    );
    assert!(res_ver
        .unwrap_err()
        .iter()
        .any(|e| e.contains("version mismatch")));

    // 2. Mismatch in source location
    let lock_tampered_source = lock_content.replace(
        "source = \"path:deps/my_lib\"",
        "source = \"path:deps/other_lib\"",
    );
    let res_source = verify_locked_dependencies(&tmp, &manifest, &lock_tampered_source);
    assert!(
        res_source.is_err(),
        "Dependency source mismatch MUST be detected"
    );
    assert!(res_source
        .unwrap_err()
        .iter()
        .any(|e| e.contains("source mismatch")));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
#[cfg(unix)]
fn test_package_store_readonly_hardening_mitigates_toctou() {
    let tmp = std::env::temp_dir().join(format!("syntra_test_toctou_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let pkg_store = tmp.join(".syntra").join("packages").join("sample_pkg");
    fs::create_dir_all(pkg_store.join("src")).unwrap();
    let src_file = pkg_store.join("src").join("main.syn");
    fs::write(&src_file, "func main(): int { return 0; }").unwrap();

    // Call hardening helper
    syntra_cli::package::protect_package_store_readonly(&pkg_store)
        .expect("Hardening package store failed");

    // Verify file is read-only (0444 on Unix)
    let meta = fs::metadata(&src_file).unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o444,
        "Installed package file must have read-only permissions (0444)"
    );

    let dir_meta = fs::metadata(pkg_store.join("src")).unwrap();
    let dir_mode = dir_meta.permissions().mode() & 0o777;
    assert_eq!(
        dir_mode, 0o555,
        "Installed package directory must be read+execute only (0555)"
    );

    // Restore permissions so cleanup succeeds
    let _ = fs::set_permissions(&src_file, fs::Permissions::from_mode(0o644));
    let _ = fs::set_permissions(pkg_store.join("src"), fs::Permissions::from_mode(0o755));
    let _ = fs::set_permissions(&pkg_store, fs::Permissions::from_mode(0o755));
    let _ = fs::remove_dir_all(&tmp);
}
