use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

/// Formats a byte slice into a lowercase hexadecimal string
fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Computes a standard SHA-256 digest of raw byte data as a lowercase hex string
pub fn hash_bytes_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    to_hex(&hasher.finalize())
}

/// Computes a deterministic SHA-256 tree hash over a directory.
/// Files are sorted lexicographically by relative path to guarantee reproducible digests.
pub fn hash_directory_tree_sha256(dir: &Path) -> Result<String, String> {
    if !dir.exists() {
        return Err(format!("Directory '{}' does not exist", dir.display()));
    }
    if dir.is_file() {
        let content =
            fs::read(dir).map_err(|e| format!("Failed to read file '{}': {}", dir.display(), e))?;
        return Ok(hash_bytes_sha256(&content));
    }

    let mut all_files = Vec::new();
    collect_source_files_recursive(dir, &mut all_files);

    // Sort relative paths lexicographically for determinism
    all_files.sort_by(|a, b| {
        let rel_a = a.strip_prefix(dir).unwrap_or(a);
        let rel_b = b.strip_prefix(dir).unwrap_or(b);
        rel_a.cmp(rel_b)
    });

    let mut hasher = Sha256::new();
    for file_path in all_files {
        let rel_path = file_path.strip_prefix(dir).unwrap_or(&file_path);
        let rel_str = rel_path.to_string_lossy();

        hasher.update(rel_str.as_bytes());
        hasher.update(b"\0");

        let content = fs::read(&file_path).map_err(|e| {
            format!(
                "Failed to read '{}' during tree hashing: {}",
                file_path.display(),
                e
            )
        })?;
        let len_bytes = (content.len() as u64).to_be_bytes();
        hasher.update(len_bytes);
        hasher.update(&content);
    }

    Ok(to_hex(&hasher.finalize()))
}

fn collect_source_files_recursive(dir: &Path, list: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let file_name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // Skip VCS, temporary files, build directories, and archives
            if file_name == ".git"
                || file_name == ".syntra"
                || file_name == "target"
                || file_name == ".DS_Store"
            {
                continue;
            }
            if file_name.ends_with(".synpkg")
                || file_name.ends_with(".o")
                || file_name.ends_with(".lock")
            {
                continue;
            }
            if p.is_dir() {
                collect_source_files_recursive(&p, list);
            } else if p.is_file() {
                list.push(p);
            }
        }
    }
}

/// Computes a cryptographic SHA-256 checksum and extra metadata for a dependency
pub fn compute_dependency_checksum(
    manifest_dir: &Path,
    dep: &syntra_frontend::PackageDependency,
) -> (String, String, String) {
    let ver = dep.version.as_deref().unwrap_or("0.1.0");

    if let Some(git) = &dep.git {
        let ref_info = if let Some(b) = &dep.branch {
            format!("#{b}")
        } else if let Some(t) = &dep.tag {
            format!("#tag:{t}")
        } else if let Some(r) = &dep.rev {
            format!("#rev:{r}")
        } else {
            "".to_string()
        };

        let (commit, tree_hash) =
            if let Ok(git_dir) = syntra_frontend::resolve_git_dependency(manifest_dir, dep) {
                let commit_out = std::process::Command::new("git")
                    .current_dir(&git_dir)
                    .args(["rev-parse", "HEAD"])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|_| "HEAD".to_string());
                let tree = hash_directory_tree_sha256(&git_dir)
                    .unwrap_or_else(|_| hash_bytes_sha256(git.as_bytes()));
                (commit_out, tree)
            } else {
                ("unknown".to_string(), hash_bytes_sha256(git.as_bytes()))
            };

        let mut hasher = Sha256::new();
        hasher.update(git.as_bytes());
        hasher.update(ref_info.as_bytes());
        hasher.update(commit.as_bytes());
        hasher.update(tree_hash.as_bytes());
        let composite_checksum = to_hex(&hasher.finalize());

        (
            format!("git+{git}{ref_info}"),
            format!("commit = \"{commit}\"\n"),
            composite_checksum,
        )
    } else if let Some(p) = &dep.path {
        let resolved_path = manifest_dir.join(p);
        let tree_hash = hash_directory_tree_sha256(&resolved_path).unwrap_or_else(|_| {
            let mut h = Sha256::new();
            h.update(p.as_bytes());
            to_hex(&h.finalize())
        });
        (format!("path:{p}"), String::new(), tree_hash)
    } else {
        // Registry or standard package
        let pkg_dir = manifest_dir
            .join(".syntra")
            .join("packages")
            .join(&dep.name);
        let tree_hash = if pkg_dir.is_dir() {
            hash_directory_tree_sha256(&pkg_dir).unwrap_or_else(|_| {
                hash_bytes_sha256(format!("registry:{}:{}", dep.name, ver).as_bytes())
            })
        } else {
            hash_bytes_sha256(format!("registry:{}:{}", dep.name, ver).as_bytes())
        };
        ("registry:syntra".to_string(), String::new(), tree_hash)
    }
}

/// Generates lockfile content for dependencies specified in manifest
pub fn generate_lock_content(
    manifest_dir: &Path,
    manifest: &syntra_frontend::PackageManifest,
) -> (String, usize) {
    let mut lock_content = format!(
        "# This file is automatically generated by Syntra package manager.\n# Do not edit manually.\n\nversion = 1\n\n[package]\nname = \"{}\"\nversion = \"{}\"\n\n",
        manifest.name, manifest.version
    );

    let mut resolved_count = 0;
    for dep in &manifest.dependencies {
        resolved_count += 1;
        let ver = dep.version.as_deref().unwrap_or("0.1.0");
        let (source, extra_meta, checksum) = compute_dependency_checksum(manifest_dir, dep);

        lock_content.push_str(&format!(
            "[[packages]]\nname = \"{}\"\nversion = \"{}\"\nsource = \"{}\"\n{}checksum = \"{}\"\n\n",
            dep.name, ver, source, extra_meta, checksum
        ));
    }

    (lock_content, resolved_count)
}

/// Verifies that dependencies in the workspace match the checksums recorded in the lockfile.
/// Returns Ok(()) on verification success, or Err(mismatches) if any dependency was modified or tampered with.
pub fn verify_locked_dependencies(
    manifest_dir: &Path,
    manifest: &syntra_frontend::PackageManifest,
    lock_content: &str,
) -> Result<(), Vec<String>> {
    let mut mismatches = Vec::new();
    let mut lock_checksums = std::collections::HashMap::new();

    let mut current_pkg_name: Option<String> = None;
    for line in lock_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name =") {
            if let Some((_, val)) = trimmed.split_once('=') {
                let name = val.trim().trim_matches('"').to_string();
                current_pkg_name = Some(name);
            }
        } else if trimmed.starts_with("checksum =") {
            if let Some((_, val)) = trimmed.split_once('=') {
                let csum = val.trim().trim_matches('"').to_string();
                if let Some(pkg) = current_pkg_name.take() {
                    lock_checksums.insert(pkg, csum);
                }
            }
        }
    }

    for dep in &manifest.dependencies {
        let (_, _, expected_checksum) = compute_dependency_checksum(manifest_dir, dep);
        match lock_checksums.get(&dep.name) {
            Some(recorded_checksum) => {
                if recorded_checksum != &expected_checksum {
                    mismatches.push(format!(
                        "Tampering or modification detected for dependency '{}': expected lockfile checksum '{}', but computed actual content checksum '{}'",
                        dep.name, recorded_checksum, expected_checksum
                    ));
                }
            }
            None => {
                mismatches.push(format!(
                    "Dependency '{}' is present in manifest but missing from lockfile.",
                    dep.name
                ));
            }
        }
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

/// Freeze and verify dependencies (syn lock [--check])
pub fn lock_dependencies(check: bool) {
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
    let (lock_content, count) = generate_lock_content(manifest_dir, &manifest);

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

    if check {
        if !lock_path.exists() {
            eprintln!(
                "❌ Error: Lockfile '{primary_lock}' does not exist. Run 'syn lock' to create it."
            );
            exit(1);
        }
        let existing = fs::read_to_string(&lock_path).unwrap_or_default();
        if let Err(mismatches) = verify_locked_dependencies(manifest_dir, &manifest, &existing) {
            for m in &mismatches {
                eprintln!("❌ Error: {m}");
            }
            eprintln!("❌ Error: Lockfile '{primary_lock}' integrity verification failed.");
            exit(1);
        }
        if existing.trim() == lock_content.trim() {
            println!(
                "✅ Lockfile '{}' is up to date and verified ({} dependenc{}).",
                primary_lock,
                count,
                if count == 1 { "y" } else { "ies" }
            );
        } else {
            eprintln!(
                "❌ Error: Lockfile '{primary_lock}' is out of sync with manifest. Run 'syn lock' to update."
            );
            exit(1);
        }
    } else {
        if let Err(e) = fs::write(&lock_path, &lock_content) {
            eprintln!("❌ Failed to write {primary_lock}: {e}");
            exit(1);
        }
        if is_syn {
            let _ = fs::write(manifest_dir.join("syntra.lock"), &lock_content);
        }
        println!(
            "🔒 Successfully locked {} dependenc{} into {}.",
            count,
            if count == 1 { "y" } else { "ies" },
            primary_lock
        );
    }
}

/// Package current project into a distributable .synpkg archive (syn pack)
pub fn pack_project(out: Option<PathBuf>, vendor: bool) {
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

    // Ensure lockfile is generated
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
    if !lock_path.exists() {
        let (lock_content, _) = generate_lock_content(manifest_dir, &manifest);
        let _ = fs::write(&lock_path, &lock_content);
    }

    let default_name = format!("{}-{}.synpkg", manifest.name, manifest.version);
    let archive_path = out.unwrap_or_else(|| PathBuf::from(&default_name));

    println!(
        "📦 Packaging Syntra project '{}' v{}...",
        manifest.name, manifest.version
    );

    // Collect files to include
    let mut files_to_pack: Vec<PathBuf> = Vec::new();
    if manifest_path.exists() {
        files_to_pack.push(manifest_path.clone());
    }
    if lock_path.exists() {
        files_to_pack.push(lock_path.clone());
    }

    // Source directories
    let src_dir = manifest_dir.join("src");
    if src_dir.is_dir() {
        collect_files_recursive(&src_dir, &mut files_to_pack);
    }
    let stdlib_dir = manifest_dir.join("stdlib");
    if stdlib_dir.is_dir() {
        collect_files_recursive(&stdlib_dir, &mut files_to_pack);
    }

    // Include README and LICENSE if present
    for doc in &["README.md", "README", "LICENSE", "LICENSE.md"] {
        let p = manifest_dir.join(doc);
        if p.is_file() {
            files_to_pack.push(p);
        }
    }

    // If vendoring requested, bundle .syntra/packages
    if vendor {
        let vendor_dir = manifest_dir.join(".syntra").join("packages");
        if vendor_dir.is_dir() {
            collect_files_recursive(&vendor_dir, &mut files_to_pack);
            println!("   Bundling vendored dependencies from .syntra/packages (offline mode)...");
        }
    }

    // Create tar.gz archive using system tar
    let mut tar_cmd = std::process::Command::new("tar");
    tar_cmd.arg("-czf").arg(&archive_path);
    for file in &files_to_pack {
        let rel = file.strip_prefix(manifest_dir).unwrap_or(file);
        tar_cmd.arg(rel);
    }

    match tar_cmd.output() {
        Ok(output) if output.status.success() => {
            let metadata = fs::metadata(&archive_path).ok();
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            let bytes = fs::read(&archive_path).unwrap_or_default();
            let hash_hex = hash_bytes_sha256(&bytes);
            println!(
                "✅ Successfully created package archive: {}",
                archive_path.display()
            );
            println!("   Files packaged: {}", files_to_pack.len());
            println!("   Archive size:   {size} bytes");
            println!("   Checksum:       sha256:{hash_hex}");
        }
        Ok(output) => {
            eprintln!(
                "❌ Failed to create archive: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            exit(1);
        }
        Err(e) => {
            eprintln!("❌ Error executing tar: {e}");
            exit(1);
        }
    }
}

fn collect_files_recursive(dir: &Path, list: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_files_recursive(&p, list);
            } else if p.is_file() {
                list.push(p);
            }
        }
    }
}
