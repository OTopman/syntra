use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

/// Formats a byte slice into a lowercase hexadecimal string
pub fn to_hex(bytes: &[u8]) -> String {
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

/// Canonical entry collected during deterministic directory tree hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEntry {
    /// Normalized relative path with forward slashes (e.g. "src/lib.syn")
    pub rel_path: String,
    /// Type of filesystem entry
    pub is_symlink: bool,
    /// Target path string if this is a symlink
    pub symlink_target: Option<String>,
    /// Whether executable permission bit is set
    pub is_executable: bool,
    /// Raw byte content for regular files
    pub content: Vec<u8>,
}

#[cfg(unix)]
pub fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
pub fn is_executable(_metadata: &fs::Metadata) -> bool {
    false
}

/// Evaluates narrow, package-root-relative exclusions.
/// Files are never excluded merely by extension (.so, .dylib, .dll, .a, .o, .lock, .exe are all included).
/// Only top-level VCS, build, and package cache directories, along with OS metadata and editor swap files, are excluded.
pub fn should_exclude(rel_path_str: &str, file_name: &str) -> bool {
    // 1. Root-relative toolchain, build, and VCS directories
    if rel_path_str == ".git"
        || rel_path_str.starts_with(".git/")
        || rel_path_str == ".syntra"
        || rel_path_str.starts_with(".syntra/")
        || rel_path_str == "target"
        || rel_path_str.starts_with("target/")
    {
        return true;
    }

    // 2. OS metadata files anywhere in tree
    if file_name == ".DS_Store" || file_name == "Thumbs.db" || file_name.starts_with("._") {
        return true;
    }

    // 3. Editor swap and backup files anywhere in tree
    if file_name.ends_with(".swp") || file_name.ends_with('~') {
        return true;
    }

    // 4. Standalone package archive at root (e.g. foo-0.1.0.synpkg)
    if rel_path_str.ends_with(".synpkg") && !rel_path_str.contains('/') {
        return true;
    }

    false
}

pub fn validate_and_normalize_symlink_target(
    root_dir: &Path,
    symlink_path: &Path,
    target: &Path,
) -> Result<String, String> {
    if target.is_absolute() {
        return Err(format!(
            "Absolute symlink rejected in package: '{}' points to '{}'",
            symlink_path.display(),
            target.display()
        ));
    }

    let parent = symlink_path.parent().unwrap_or(root_dir);
    let mut stack = Vec::new();

    if let Ok(rel_parent) = parent.strip_prefix(root_dir) {
        for comp in rel_parent.components() {
            if let std::path::Component::Normal(c) = comp {
                let s = c
                    .to_str()
                    .ok_or_else(|| "Non-UTF8 path component in symlink parent".to_string())?;
                stack.push(s.to_string());
            }
        }
    }

    for comp in target.components() {
        match comp {
            std::path::Component::Normal(c) => {
                let s = c
                    .to_str()
                    .ok_or_else(|| "Non-UTF8 path component in symlink target".to_string())?;
                stack.push(s.to_string());
            }
            std::path::Component::ParentDir => {
                if stack.is_empty() {
                    return Err(format!(
                        "Path traversal attempt: symlink '{}' escapes package root with target '{}'",
                        symlink_path.display(),
                        target.display()
                    ));
                }
                stack.pop();
            }
            std::path::Component::CurDir => {}
            _ => {
                return Err(format!(
                    "Unsupported path component in symlink target '{}'",
                    target.display()
                ));
            }
        }
    }

    let target_str = target
        .to_str()
        .ok_or_else(|| "Non-UTF8 symlink target".to_string())?
        .replace('\\', "/");
    Ok(target_str)
}

fn collect_canonical_entries(
    current_dir: &Path,
    root_dir: &Path,
    list: &mut Vec<CanonicalEntry>,
) -> Result<(), String> {
    let read_dir = fs::read_dir(current_dir).map_err(|e| {
        format!(
            "Failed to read directory '{}' during tree hashing: {e}",
            current_dir.display()
        )
    })?;

    for entry_res in read_dir {
        let entry = entry_res.map_err(|e| {
            format!(
                "Failed to read directory entry in '{}' during tree hashing: {e}",
                current_dir.display()
            )
        })?;

        let path = entry.path();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let rel_path = path
            .strip_prefix(root_dir)
            .map_err(|e| format!("Failed to strip prefix for '{}': {e}", path.display()))?;

        // Normalize relative path components to UTF-8 with forward slashes
        let mut components = Vec::new();
        for comp in rel_path.components() {
            match comp {
                std::path::Component::Normal(c) => {
                    let s = c.to_str().ok_or_else(|| {
                        format!("Non-UTF8 path component in '{}'", path.display())
                    })?;
                    components.push(s);
                }
                std::path::Component::ParentDir => {
                    return Err(format!(
                        "Path traversal component '..' found in relative path '{}'",
                        path.display()
                    ));
                }
                std::path::Component::CurDir => {}
                _ => {
                    return Err(format!("Invalid path component in '{}'", path.display()));
                }
            }
        }
        let normalized_rel = components.join("/");

        if should_exclude(&normalized_rel, &file_name) {
            continue;
        }

        let sym_meta = fs::symlink_metadata(&path).map_err(|e| {
            format!(
                "Failed to read metadata for '{}' during tree hashing: {e}",
                path.display()
            )
        })?;

        let ft = sym_meta.file_type();
        if ft.is_dir() {
            collect_canonical_entries(&path, root_dir, list)?;
        } else if ft.is_symlink() {
            let target = fs::read_link(&path).map_err(|e| {
                format!(
                    "Failed to read symlink target for '{}': {e}",
                    path.display()
                )
            })?;

            let normalized_target =
                validate_and_normalize_symlink_target(root_dir, &path, &target)?;

            list.push(CanonicalEntry {
                rel_path: normalized_rel,
                is_symlink: true,
                symlink_target: Some(normalized_target),
                is_executable: false,
                content: Vec::new(),
            });
        } else if ft.is_file() {
            let is_exec = is_executable(&sym_meta);
            let content = fs::read(&path).map_err(|e| {
                format!(
                    "Failed to read file '{}' during tree hashing: {e}",
                    path.display()
                )
            })?;

            list.push(CanonicalEntry {
                rel_path: normalized_rel,
                is_symlink: false,
                symlink_target: None,
                is_executable: is_exec,
                content,
            });
        } else {
            return Err(format!(
                "Unsupported filesystem object '{}' (special device, socket, or fifo) in package tree",
                path.display()
            ));
        }
    }

    Ok(())
}

/// Computes a deterministic canonical SHA-256 tree hash over a directory.
/// Accounts for normalized relative paths, file contents, entry types, symlink targets,
/// and executable permissions, with strict lexicographical ordering.
pub fn hash_directory_tree_sha256(dir: &Path) -> Result<String, String> {
    if !dir.exists() {
        return Err(format!("Directory '{}' does not exist", dir.display()));
    }
    if dir.is_file() {
        let content =
            fs::read(dir).map_err(|e| format!("Failed to read file '{}': {e}", dir.display()))?;
        return Ok(hash_bytes_sha256(&content));
    }

    let mut entries = Vec::new();
    collect_canonical_entries(dir, dir, &mut entries)?;

    // Strict lexicographical sort by normalized relative path
    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    let mut hasher = Sha256::new();
    for entry in entries {
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
    }

    Ok(to_hex(&hasher.finalize()))
}

/// Hardens an installed package tree by setting read-only permissions on all files and directories,
/// mitigating Time-of-Check / Time-of-Use (TOCTOU) mutations between verification and compilation.
pub fn protect_package_store_readonly(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(|e| {
        format!(
            "Failed to read directory '{}' during permission hardening: {e}",
            dir.display()
        )
    })?;

    for entry_res in entries {
        let entry = entry_res
            .map_err(|e| format!("Directory traversal error during permission hardening: {e}"))?;
        let p = entry.path();
        let sym_meta = fs::symlink_metadata(&p)
            .map_err(|e| format!("Metadata read error during permission hardening: {e}"))?;

        if sym_meta.is_dir() {
            protect_package_store_readonly(&p)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o555));
            }
        } else if sym_meta.is_file() {
            let mut perms = sym_meta.permissions();
            perms.set_readonly(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = if is_executable(&sym_meta) {
                    0o555
                } else {
                    0o444
                };
                let _ = fs::set_permissions(&p, fs::Permissions::from_mode(mode));
            }
            #[cfg(not(unix))]
            {
                let _ = fs::set_permissions(&p, perms);
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o555));
    }

    Ok(())
}

/// Parses and unescapes a TOML string literal according to standard TOML string escaping rules.
pub fn parse_toml_string(raw: &str, line_num: usize) -> Result<String, String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') || trimmed.len() < 2 {
        return Err(format!(
            "Line {line_num}: Expected string literal enclosed in double quotes, found '{trimmed}'"
        ));
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            let esc = chars.next().ok_or_else(|| {
                format!("Line {line_num}: Incomplete escape sequence in string literal")
            })?;
            match esc {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'b' => out.push('\x08'),
                'f' => out.push('\x0C'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let mut hex = String::with_capacity(4);
                    for _ in 0..4 {
                        let h = chars.next().ok_or_else(|| {
                            format!("Line {line_num}: Incomplete \\u unicode escape")
                        })?;
                        hex.push(h);
                    }
                    let code = u32::from_str_radix(&hex, 16).map_err(|_| {
                        format!("Line {line_num}: Invalid \\u unicode hex sequence '{hex}'")
                    })?;
                    let ch = char::from_u32(code).ok_or_else(|| {
                        format!("Line {line_num}: Invalid unicode codepoint 'U+{code:04X}'")
                    })?;
                    out.push(ch);
                }
                'U' => {
                    let mut hex = String::with_capacity(8);
                    for _ in 0..8 {
                        let h = chars.next().ok_or_else(|| {
                            format!("Line {line_num}: Incomplete \\U unicode escape")
                        })?;
                        hex.push(h);
                    }
                    let code = u32::from_str_radix(&hex, 16).map_err(|_| {
                        format!("Line {line_num}: Invalid \\U unicode hex sequence '{hex}'")
                    })?;
                    let ch = char::from_u32(code).ok_or_else(|| {
                        format!("Line {line_num}: Invalid unicode codepoint 'U+{code:08X}'")
                    })?;
                    out.push(ch);
                }
                other => {
                    return Err(format!(
                        "Line {line_num}: Invalid TOML escape sequence '\\{other}'"
                    ));
                }
            }
        } else if c.is_control() && c != '\t' {
            return Err(format!(
                "Line {line_num}: Unescaped control character in string literal"
            ));
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// Structured representation of a package recorded in the lockfile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockPackageEntry {
    pub name: String,
    pub version: String,
    pub source: String,
    pub commit: Option<String>,
    pub checksum: String,
}

#[derive(Default)]
struct LockPackageEntryBuilder {
    name: Option<String>,
    version: Option<String>,
    source: Option<String>,
    commit: Option<String>,
    checksum: Option<String>,
    seen_keys: HashSet<String>,
}

impl LockPackageEntryBuilder {
    fn build(self, line_num: usize) -> Result<LockPackageEntry, String> {
        let name = self
            .name
            .ok_or_else(|| format!("Line {line_num}: Missing 'name' in [[packages]]"))?;
        let version = self
            .version
            .ok_or_else(|| format!("Line {line_num}: Missing 'version' for package '{name}'"))?;
        let source = self
            .source
            .ok_or_else(|| format!("Line {line_num}: Missing 'source' for package '{name}'"))?;
        let checksum = self
            .checksum
            .ok_or_else(|| format!("Line {line_num}: Missing 'checksum' for package '{name}'"))?;

        let clean_csum = if let Some(stripped) = checksum.strip_prefix("sha256:") {
            stripped
        } else {
            &checksum
        };
        if clean_csum.len() != 64 || !clean_csum.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Line {line_num}: Malformed SHA-256 checksum '{checksum}' for package '{name}' (must be 64 hexadecimal characters)"
            ));
        }

        if source.starts_with("git+") {
            if let Some(commit) = &self.commit {
                if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(format!(
                        "Line {line_num}: Invalid git commit hash '{commit}' for package '{name}' (must be 40 hexadecimal characters)"
                    ));
                }
            } else {
                return Err(format!(
                    "Line {line_num}: Missing 'commit' for git dependency '{name}'"
                ));
            }
        }

        Ok(LockPackageEntry {
            name,
            version,
            source,
            commit: self.commit,
            checksum: clean_csum.to_lowercase(),
        })
    }
}

/// Root package declaration in lockfile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockRootPackage {
    pub name: String,
    pub version: String,
}

/// Fully parsed, validated schema representation of a Syntra lockfile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lockfile {
    pub version: u32,
    pub package: LockRootPackage,
    pub packages: Vec<LockPackageEntry>,
}

impl Lockfile {
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut version = None;
        let mut root_name = None;
        let mut root_version = None;
        let mut packages = Vec::new();

        enum Section {
            TopLevel,
            Package,
            Packages,
        }

        let mut current_section = Section::TopLevel;
        let mut current_pkg: Option<LockPackageEntryBuilder> = None;
        let mut seen_packages = HashSet::new();
        let mut seen_top_level_keys = HashSet::new();
        let mut package_section_seen = false;
        let mut package_seen_keys = HashSet::new();

        for (line_idx, line) in input.lines().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed == "[package]" {
                if package_section_seen {
                    return Err(format!(
                        "Line {line_num}: Duplicate [package] section in lockfile"
                    ));
                }
                package_section_seen = true;

                if let Some(builder) = current_pkg.take() {
                    let entry = builder.build(line_num)?;
                    if !seen_packages.insert(entry.name.clone()) {
                        return Err(format!(
                            "Line {line_num}: Duplicate package entry in lockfile: '{}'",
                            entry.name
                        ));
                    }
                    packages.push(entry);
                }
                current_section = Section::Package;
                continue;
            }

            if trimmed == "[[packages]]" {
                if let Some(builder) = current_pkg.take() {
                    let entry = builder.build(line_num)?;
                    if !seen_packages.insert(entry.name.clone()) {
                        return Err(format!(
                            "Line {line_num}: Duplicate package entry in lockfile: '{}'",
                            entry.name
                        ));
                    }
                    packages.push(entry);
                }
                current_section = Section::Packages;
                current_pkg = Some(LockPackageEntryBuilder::default());
                continue;
            }

            let (key, val) = trimmed.split_once('=').ok_or_else(|| {
                format!(
                    "Line {line_num}: Invalid syntax in lockfile: expected key = value, found '{trimmed}'"
                )
            })?;
            let key = key.trim();
            let val = val.trim();

            match current_section {
                Section::TopLevel => {
                    if !seen_top_level_keys.insert(key.to_string()) {
                        return Err(format!(
                            "Line {line_num}: Duplicate top-level key '{key}' in lockfile"
                        ));
                    }
                    if key == "version" {
                        let v = val.parse::<u32>().map_err(|_| {
                            format!("Line {line_num}: Invalid schema version '{val}'")
                        })?;
                        if v != 1 {
                            return Err(format!(
                                "Line {line_num}: Unsupported lockfile schema version '{v}'; expected version = 1"
                            ));
                        }
                        version = Some(v);
                    } else {
                        return Err(format!(
                            "Line {line_num}: Unexpected top-level key '{key}' in lockfile"
                        ));
                    }
                }
                Section::Package => {
                    if !package_seen_keys.insert(key.to_string()) {
                        return Err(format!(
                            "Line {line_num}: Duplicate key '{key}' in [package] section"
                        ));
                    }
                    let parsed_val = parse_toml_string(val, line_num)?;
                    match key {
                        "name" => root_name = Some(parsed_val),
                        "version" => root_version = Some(parsed_val),
                        _ => {
                            return Err(format!(
                                "Line {line_num}: Unknown key '{key}' in [package] section"
                            ))
                        }
                    }
                }
                Section::Packages => {
                    let builder = current_pkg.as_mut().ok_or_else(|| {
                        format!(
                            "Line {line_num}: Key '{key}' found outside of [[packages]] section"
                        )
                    })?;
                    if !builder.seen_keys.insert(key.to_string()) {
                        return Err(format!(
                            "Line {line_num}: Duplicate key '{key}' in [[packages]] block"
                        ));
                    }
                    let parsed_val = parse_toml_string(val, line_num)?;
                    match key {
                        "name" => builder.name = Some(parsed_val),
                        "version" => builder.version = Some(parsed_val),
                        "source" => builder.source = Some(parsed_val),
                        "commit" => builder.commit = Some(parsed_val),
                        "checksum" => builder.checksum = Some(parsed_val),
                        _ => {
                            return Err(format!(
                                "Line {line_num}: Unknown key '{key}' in [[packages]] section"
                            ))
                        }
                    }
                }
            }
        }

        if let Some(builder) = current_pkg.take() {
            let entry = builder.build(input.lines().count())?;
            if !seen_packages.insert(entry.name.clone()) {
                return Err(format!(
                    "Duplicate package entry in lockfile: '{}'",
                    entry.name
                ));
            }
            packages.push(entry);
        }

        let version =
            version.ok_or_else(|| "Missing required 'version' field in lockfile".to_string())?;
        let root_name = root_name.ok_or_else(|| {
            "Missing required 'name' in [package] section of lockfile".to_string()
        })?;
        let root_version = root_version.ok_or_else(|| {
            "Missing required 'version' in [package] section of lockfile".to_string()
        })?;

        Ok(Lockfile {
            version,
            package: LockRootPackage {
                name: root_name,
                version: root_version,
            },
            packages,
        })
    }

    pub fn to_canonical_string(&self) -> String {
        let mut out = String::new();
        out.push_str("# This file is automatically generated by Syntra package manager.\n");
        out.push_str("# Do not edit manually.\n\n");
        out.push_str(&format!("version = {}\n\n", self.version));
        out.push_str("[package]\n");
        out.push_str(&format!("name = \"{}\"\n", self.package.name));
        out.push_str(&format!("version = \"{}\"\n\n", self.package.version));

        let mut pkgs = self.packages.clone();
        pkgs.sort_by(|a, b| a.name.cmp(&b.name));

        for p in &pkgs {
            out.push_str("[[packages]]\n");
            out.push_str(&format!("name = \"{}\"\n", p.name));
            out.push_str(&format!("version = \"{}\"\n", p.version));
            out.push_str(&format!("source = \"{}\"\n", p.source));
            if let Some(commit) = &p.commit {
                out.push_str(&format!("commit = \"{commit}\"\n"));
            }
            out.push_str(&format!("checksum = \"{}\"\n\n", p.checksum));
        }
        out
    }
}

/// Computes a cryptographic SHA-256 checksum and metadata for a dependency.
/// NEVER falls back to URL, name, or metadata hashing if actual package contents
/// cannot be inspected.
pub fn compute_dependency_checksum(
    manifest_dir: &Path,
    dep: &syntra_frontend::PackageDependency,
) -> Result<LockPackageEntry, String> {
    let ver = dep.version.as_deref().unwrap_or("0.1.0");

    if let Some(git) = &dep.git {
        let ref_info = if let Some(b) = &dep.branch {
            format!("#{b}")
        } else if let Some(t) = &dep.tag {
            format!("#tag:{t}")
        } else if let Some(r) = &dep.rev {
            format!("#rev:{r}")
        } else {
            String::new()
        };

        let git_dir = syntra_frontend::resolve_git_dependency(manifest_dir, dep)
            .map_err(|e| format!("Failed to resolve git dependency '{}': {e}", dep.name))?;

        if !git_dir.is_dir() {
            return Err(format!(
                "Git dependency directory for '{}' does not exist at '{}'",
                dep.name,
                git_dir.display()
            ));
        }

        let commit_out = std::process::Command::new("git")
            .current_dir(&git_dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| {
                format!(
                    "Failed to execute 'git rev-parse HEAD' for dependency '{}': {e}",
                    dep.name
                )
            })?;

        if !commit_out.status.success() {
            return Err(format!(
                "'git rev-parse HEAD' failed for dependency '{}': {}",
                dep.name,
                String::from_utf8_lossy(&commit_out.stderr).trim()
            ));
        }

        let commit = String::from_utf8_lossy(&commit_out.stdout)
            .trim()
            .to_string();
        if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid git commit hash '{commit}' returned for dependency '{}'",
                dep.name
            ));
        }

        let tree_hash = hash_directory_tree_sha256(&git_dir).map_err(|e| {
            format!(
                "Failed to compute content hash for git dependency '{}': {e}",
                dep.name
            )
        })?;

        Ok(LockPackageEntry {
            name: dep.name.clone(),
            version: ver.to_string(),
            source: format!("git+{git}{ref_info}"),
            commit: Some(commit),
            checksum: tree_hash,
        })
    } else if let Some(p) = &dep.path {
        let resolved_path = manifest_dir.join(p);
        if !resolved_path.exists() {
            return Err(format!(
                "Local path dependency '{}' does not exist at '{}'",
                dep.name,
                resolved_path.display()
            ));
        }
        if !resolved_path.is_dir() {
            return Err(format!(
                "Local path dependency '{}' at '{}' is not a directory",
                dep.name,
                resolved_path.display()
            ));
        }

        let tree_hash = hash_directory_tree_sha256(&resolved_path).map_err(|e| {
            format!(
                "Failed to compute content hash for path dependency '{}': {e}",
                dep.name
            )
        })?;

        Ok(LockPackageEntry {
            name: dep.name.clone(),
            version: ver.to_string(),
            source: format!("path:{p}"),
            commit: None,
            checksum: tree_hash,
        })
    } else {
        let pkg_dir = manifest_dir
            .join(".syntra")
            .join("packages")
            .join(&dep.name);

        if !pkg_dir.is_dir() {
            return Err(format!(
                "Registry package '{}' is not installed in '{}'. Run 'syn install' first.",
                dep.name,
                pkg_dir.display()
            ));
        }

        let tree_hash = hash_directory_tree_sha256(&pkg_dir).map_err(|e| {
            format!(
                "Failed to compute content hash for registry package '{}': {e}",
                dep.name
            )
        })?;

        Ok(LockPackageEntry {
            name: dep.name.clone(),
            version: ver.to_string(),
            source: "registry:syntra".to_string(),
            commit: None,
            checksum: tree_hash,
        })
    }
}

/// Generates lockfile content for dependencies specified in manifest.
/// Returns Ok((lock_content, resolved_count)) on success, or Err on failure.
pub fn generate_lock_content(
    manifest_dir: &Path,
    manifest: &syntra_frontend::PackageManifest,
) -> Result<(String, usize), String> {
    let mut packages = Vec::with_capacity(manifest.dependencies.len());

    for dep in &manifest.dependencies {
        let entry = compute_dependency_checksum(manifest_dir, dep)?;
        packages.push(entry);
    }

    let lockfile = Lockfile {
        version: 1,
        package: LockRootPackage {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
        },
        packages,
    };

    let count = lockfile.packages.len();
    Ok((lockfile.to_canonical_string(), count))
}

/// Verifies that dependencies in the workspace match the complete recorded dependency identity
/// (Root name & version, dependency name, version, source URI, Git revision, and canonical SHA-256 content checksum).
/// Returns Ok(()) on verification success, or Err(mismatches) if any discrepancy is detected.
pub fn verify_locked_dependencies(
    manifest_dir: &Path,
    manifest: &syntra_frontend::PackageManifest,
    lock_content: &str,
) -> Result<(), Vec<String>> {
    let mut mismatches = Vec::new();

    let lockfile = match Lockfile::parse(lock_content) {
        Ok(l) => l,
        Err(e) => {
            return Err(vec![format!("Lockfile schema validation failed: {e}")]);
        }
    };

    // 0. Verify root package identity matches manifest
    if lockfile.package.name != manifest.name {
        mismatches.push(format!(
            "Lockfile root package name mismatch: lockfile records '{}', but manifest requires '{}'",
            lockfile.package.name, manifest.name
        ));
    }
    if lockfile.package.version != manifest.version {
        mismatches.push(format!(
            "Lockfile root package version mismatch: lockfile records '{}', but manifest requires '{}'",
            lockfile.package.version, manifest.version
        ));
    }

    let mut locked_map: HashMap<String, LockPackageEntry> = HashMap::new();
    for pkg in lockfile.packages {
        locked_map.insert(pkg.name.clone(), pkg);
    }

    // 1. Check for missing dependencies
    for dep in &manifest.dependencies {
        if !locked_map.contains_key(&dep.name) {
            mismatches.push(format!(
                "Dependency '{}' is present in manifest but missing from lockfile.",
                dep.name
            ));
        }
    }

    // 2. Check for unexpected dependencies
    let manifest_dep_names: HashSet<&str> = manifest
        .dependencies
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    for pkg_name in locked_map.keys() {
        if !manifest_dep_names.contains(pkg_name.as_str()) {
            mismatches.push(format!(
                "Lockfile contains unexpected dependency '{pkg_name}' not declared in package manifest."
            ));
        }
    }

    // 3. Verify each declared dependency's complete identity and content checksum
    for dep in &manifest.dependencies {
        let locked_entry = match locked_map.get(&dep.name) {
            Some(e) => e,
            None => continue,
        };

        let actual_entry = match compute_dependency_checksum(manifest_dir, dep) {
            Ok(entry) => entry,
            Err(e) => {
                mismatches.push(format!("Failed to inspect dependency '{}': {e}", dep.name));
                continue;
            }
        };

        // 3a. Verify dependency version
        if locked_entry.version != actual_entry.version {
            mismatches.push(format!(
                "Dependency '{}' version mismatch: lockfile recorded '{}', but resolved dependency version is '{}'",
                dep.name, locked_entry.version, actual_entry.version
            ));
        }

        // 3b. Verify dependency source
        if locked_entry.source != actual_entry.source {
            mismatches.push(format!(
                "Dependency '{}' source mismatch: lockfile recorded '{}', but resolved dependency source is '{}'",
                dep.name, locked_entry.source, actual_entry.source
            ));
        }

        // 3c. Verify content checksum
        if locked_entry.checksum != actual_entry.checksum {
            mismatches.push(format!(
                "Tampering or modification detected for dependency '{}': expected lockfile checksum '{}', but computed actual content checksum '{}'",
                dep.name, locked_entry.checksum, actual_entry.checksum
            ));
        }

        // 3d. Verify Git commit (for Git dependencies)
        if dep.git.is_some() {
            match (&locked_entry.commit, &actual_entry.commit) {
                (Some(expected_commit), Some(actual_commit)) => {
                    if expected_commit != actual_commit {
                        mismatches.push(format!(
                            "Git revision mismatch for dependency '{}': lockfile recorded commit '{}', but repository is at '{}'",
                            dep.name, expected_commit, actual_commit
                        ));
                    }
                }
                (None, Some(actual_commit)) => {
                    mismatches.push(format!(
                        "Lockfile missing commit hash for Git dependency '{}' (current HEAD is '{}')",
                        dep.name, actual_commit
                    ));
                }
                _ => {}
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
                "❌ Failed to read manifest {}: {e}",
                manifest_path.display()
            );
            exit(1);
        }
    };

    let manifest = syntra_frontend::PackageManifest::parse(&content);
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));

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
        let existing = match fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Error: Failed to read lockfile '{primary_lock}': {e}");
                exit(1);
            }
        };

        if let Err(mismatches) = verify_locked_dependencies(manifest_dir, &manifest, &existing) {
            for m in &mismatches {
                eprintln!("❌ Error: {m}");
            }
            eprintln!("❌ Error: Lockfile '{primary_lock}' integrity verification failed.");
            exit(1);
        }

        match generate_lock_content(manifest_dir, &manifest) {
            Ok((expected_lock, count)) => {
                if existing.trim() == expected_lock.trim() {
                    println!(
                        "✅ Lockfile '{}' is up to date and verified ({} dependenc{}).",
                        primary_lock,
                        count,
                        if count == 1 { "y" } else { "ies" }
                    );
                } else {
                    eprintln!(
                        "❌ Error: Lockfile '{primary_lock}' is out of sync with current package tree or manifest. Run 'syn lock' to update."
                    );
                    exit(1);
                }
            }
            Err(e) => {
                eprintln!("❌ Error verifying dependencies against package tree: {e}");
                exit(1);
            }
        }
    } else {
        let (lock_content, count) = match generate_lock_content(manifest_dir, &manifest) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("❌ Error generating lockfile: {e}");
                exit(1);
            }
        };

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
                "❌ Failed to read manifest {}: {e}",
                manifest_path.display()
            );
            exit(1);
        }
    };

    let manifest = syntra_frontend::PackageManifest::parse(&content);
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));

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
        let (lock_content, _) = match generate_lock_content(manifest_dir, &manifest) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("❌ Failed to generate lockfile before packaging: {e}");
                exit(1);
            }
        };
        if let Err(e) = fs::write(&lock_path, &lock_content) {
            eprintln!("❌ Failed to write lockfile before packaging: {e}");
            exit(1);
        }
    }

    let default_name = format!("{}-{}.synpkg", manifest.name, manifest.version);
    let archive_path = out.unwrap_or_else(|| PathBuf::from(&default_name));

    println!(
        "📦 Packaging Syntra project '{}' v{}...",
        manifest.name, manifest.version
    );

    let mut files_to_pack: Vec<PathBuf> = Vec::new();
    if manifest_path.exists() {
        files_to_pack.push(manifest_path.clone());
    }
    if lock_path.exists() {
        files_to_pack.push(lock_path.clone());
    }

    let src_dir = manifest_dir.join("src");
    if src_dir.is_dir() {
        if let Err(e) = collect_files_recursive(&src_dir, &mut files_to_pack) {
            eprintln!("❌ Error reading source files: {e}");
            exit(1);
        }
    }
    let stdlib_dir = manifest_dir.join("stdlib");
    if stdlib_dir.is_dir() {
        if let Err(e) = collect_files_recursive(&stdlib_dir, &mut files_to_pack) {
            eprintln!("❌ Error reading stdlib files: {e}");
            exit(1);
        }
    }

    for doc in &["README.md", "README", "LICENSE", "LICENSE.md"] {
        let p = manifest_dir.join(doc);
        if p.is_file() {
            files_to_pack.push(p);
        }
    }

    if vendor {
        let vendor_dir = manifest_dir.join(".syntra").join("packages");
        if vendor_dir.is_dir() {
            if let Err(e) = collect_files_recursive(&vendor_dir, &mut files_to_pack) {
                eprintln!("❌ Error reading vendor directory: {e}");
                exit(1);
            }
            println!("   Bundling vendored dependencies from .syntra/packages (offline mode)...");
        }
    }

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

fn collect_files_recursive(dir: &Path, list: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {e}", dir.display()))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| format!("Failed to read directory entry in '{}': {e}", dir.display()))?;
        let p = entry.path();
        let sym_meta = fs::symlink_metadata(&p)
            .map_err(|e| format!("Failed to read metadata for '{}': {e}", p.display()))?;
        if sym_meta.is_dir() {
            collect_files_recursive(&p, list)?;
        } else if sym_meta.is_file() || sym_meta.is_symlink() {
            list.push(p);
        }
    }
    Ok(())
}
