use std::fs;
use syntra_cli::package::{
    generate_lock_content, hash_bytes_sha256, hash_directory_tree_sha256,
    verify_locked_dependencies,
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
    let (lock_content, count) = generate_lock_content(&tmp, &manifest);
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

    let (lock_content, _) = generate_lock_content(&tmp, &manifest);
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
