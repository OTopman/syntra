use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use syntra_diagnostics::{Diagnostic, Span};
use syntra_syntax::ast::{Item, NamespaceDecl, Program};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageDependency {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
}

impl PackageDependency {
    pub fn is_git(&self) -> bool {
        self.git.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptimizationConfig {
    pub pgo: Option<String>,
    pub telemetry_mode: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildConfig {
    pub mode: Option<String>,
    pub memory_domains: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkConfig {
    pub tls_provider: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub dependencies: Vec<PackageDependency>,
    pub optimization: OptimizationConfig,
    pub build: BuildConfig,
    pub network: NetworkConfig,
}

impl PackageManifest {
    pub fn parse(content: &str) -> Self {
        // Detect if content is TOML format (starts with [package] or [dependencies])
        if content.contains("[package]") || content.contains("[dependencies]") {
            return Self::parse_toml(content);
        }
        Self::parse_syntra_dsl(content)
    }

    pub fn parse_toml(content: &str) -> Self {
        let mut manifest = Self::default();
        let mut section = "package";

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed == "[package]" {
                section = "package";
                continue;
            }
            if trimmed == "[dependencies]" {
                section = "dependencies";
                continue;
            }
            if trimmed == "[optimization]" {
                section = "optimization";
                continue;
            }
            if trimmed == "[build]" {
                section = "build";
                continue;
            }
            if trimmed == "[network]" {
                section = "network";
                continue;
            }
            if trimmed.starts_with('[') {
                section = "other";
                continue;
            }

            match section {
                "package" => {
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"');
                        match k {
                            "name" => manifest.name = v.to_string(),
                            "version" => manifest.version = v.to_string(),
                            "edition" => manifest.edition = v.to_string(),
                            _ => {}
                        }
                    }
                }
                "dependencies" => {
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let dep_name = k.trim().to_string();
                        let val = v.trim();
                        if val.starts_with('{') && val.ends_with('}') {
                            let inner = &val[1..val.len() - 1];
                            let mut dep = PackageDependency {
                                name: dep_name,
                                version: None,
                                path: None,
                                git: None,
                                branch: None,
                                tag: None,
                                rev: None,
                            };
                            for part in inner.split(',') {
                                if let Some((pk, pv)) = part.split_once('=') {
                                    let pk = pk.trim();
                                    let pv = pv.trim().trim_matches('"').trim_matches('\'');
                                    if pk == "path" {
                                        dep.path = Some(pv.to_string());
                                    } else if pk == "version" {
                                        dep.version = Some(pv.to_string());
                                    } else if pk == "git" {
                                        dep.git = Some(pv.to_string());
                                    } else if pk == "branch" {
                                        dep.branch = Some(pv.to_string());
                                    } else if pk == "tag" {
                                        dep.tag = Some(pv.to_string());
                                    } else if pk == "rev" {
                                        dep.rev = Some(pv.to_string());
                                    }
                                }
                            }
                            manifest.dependencies.push(dep);
                        } else {
                            let ver = val.trim_matches('"').trim_matches('\'').to_string();
                            if ver.starts_with("git+")
                                || ver.starts_with("https://")
                                || ver.starts_with("git@")
                            {
                                manifest.dependencies.push(PackageDependency {
                                    name: dep_name,
                                    version: None,
                                    path: None,
                                    git: Some(ver.strip_prefix("git+").unwrap_or(&ver).to_string()),
                                    branch: None,
                                    tag: None,
                                    rev: None,
                                });
                            } else {
                                manifest.dependencies.push(PackageDependency {
                                    name: dep_name,
                                    version: Some(ver),
                                    path: None,
                                    git: None,
                                    branch: None,
                                    tag: None,
                                    rev: None,
                                });
                            }
                        }
                    }
                }
                "optimization" => {
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"').trim_matches('\'');
                        match k {
                            "pgo" => manifest.optimization.pgo = Some(v.to_string()),
                            "telemetry_mode" => {
                                manifest.optimization.telemetry_mode = Some(v.to_string())
                            }
                            _ => {}
                        }
                    }
                }
                "build" => {
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"').trim_matches('\'');
                        match k {
                            "mode" => manifest.build.mode = Some(v.to_string()),
                            "memory_domains" => manifest.build.memory_domains = Some(v.to_string()),
                            _ => {}
                        }
                    }
                }
                "network" => {
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"').trim_matches('\'');
                        if k == "tls_provider" {
                            manifest.network.tls_provider = Some(v.to_string())
                        }
                    }
                }
                _ => {}
            }
        }

        manifest
    }

    pub fn parse_syntra_dsl(content: &str) -> Self {
        let mut manifest = Self {
            edition: "2026".to_string(),
            ..Default::default()
        };

        let mut in_deps = false;
        let mut current_block = "root";

        for line in content.lines() {
            let trimmed = line.trim();
            // Skip comments and empty lines
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
            {
                continue;
            }

            if trimmed.contains("dependencies") && (trimmed.contains('[') || trimmed.contains('='))
            {
                in_deps = true;
                if trimmed.contains(']') {
                    in_deps = false;
                }
                continue;
            }

            if in_deps {
                if trimmed.starts_with(']') || trimmed.starts_with("];") {
                    in_deps = false;
                    continue;
                }

                // Parse dependency entry:
                // e.g. Dependency.version("math_lib", "1.0.0"),
                // e.g. Dependency.path("helpers", "./libs/helpers"),
                // e.g. Dependency.package("utils", "0.2.0", "./libs/utils"),
                // e.g. "math_lib: 1.0.0",
                if let Some(pos) = trimmed.find("Dependency.version(") {
                    let rest = &trimmed[pos + 19..];
                    if let Some(end) = rest.find(')') {
                        let inner = &rest[..end];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() >= 2 {
                            let name = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let ver = parts[1].trim().trim_matches('"').trim_matches('\'');
                            manifest.dependencies.push(PackageDependency {
                                name: name.to_string(),
                                version: Some(ver.to_string()),
                                path: None,
                                ..Default::default()
                            });
                        }
                    }
                } else if let Some(pos) = trimmed.find("Dependency.path(") {
                    let rest = &trimmed[pos + 16..];
                    if let Some(end) = rest.find(')') {
                        let inner = &rest[..end];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() >= 2 {
                            let name = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let p = parts[1].trim().trim_matches('"').trim_matches('\'');
                            manifest.dependencies.push(PackageDependency {
                                name: name.to_string(),
                                version: None,
                                path: Some(p.to_string()),
                                ..Default::default()
                            });
                        }
                    }
                } else if let Some(pos) = trimmed.find("Dependency.package(") {
                    let rest = &trimmed[pos + 19..];
                    if let Some(end) = rest.find(')') {
                        let inner = &rest[..end];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() >= 3 {
                            let name = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let ver = parts[1].trim().trim_matches('"').trim_matches('\'');
                            let p = parts[2].trim().trim_matches('"').trim_matches('\'');
                            manifest.dependencies.push(PackageDependency {
                                name: name.to_string(),
                                version: Some(ver.to_string()),
                                path: Some(p.to_string()),
                                ..Default::default()
                            });
                        }
                    }
                } else if let Some(pos) = trimmed.find("Dependency.git(") {
                    let rest = &trimmed[pos + 15..];
                    if let Some(end) = rest.find(')') {
                        let inner = &rest[..end];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() >= 2 {
                            let name = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let url = parts[1].trim().trim_matches('"').trim_matches('\'');
                            manifest.dependencies.push(PackageDependency {
                                name: name.to_string(),
                                git: Some(url.to_string()),
                                ..Default::default()
                            });
                        }
                    }
                } else if let Some(pos) = trimmed.find("Dependency.gitBranch(") {
                    let rest = &trimmed[pos + 21..];
                    if let Some(end) = rest.find(')') {
                        let inner = &rest[..end];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() >= 3 {
                            let name = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let url = parts[1].trim().trim_matches('"').trim_matches('\'');
                            let branch = parts[2].trim().trim_matches('"').trim_matches('\'');
                            manifest.dependencies.push(PackageDependency {
                                name: name.to_string(),
                                git: Some(url.to_string()),
                                branch: Some(branch.to_string()),
                                ..Default::default()
                            });
                        }
                    }
                } else if let Some(pos) = trimmed.find("Dependency.gitTag(") {
                    let rest = &trimmed[pos + 18..];
                    if let Some(end) = rest.find(')') {
                        let inner = &rest[..end];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() >= 3 {
                            let name = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let url = parts[1].trim().trim_matches('"').trim_matches('\'');
                            let tag = parts[2].trim().trim_matches('"').trim_matches('\'');
                            manifest.dependencies.push(PackageDependency {
                                name: name.to_string(),
                                git: Some(url.to_string()),
                                tag: Some(tag.to_string()),
                                ..Default::default()
                            });
                        }
                    }
                } else if let Some(pos) = trimmed.find("Dependency.gitRev(") {
                    let rest = &trimmed[pos + 18..];
                    if let Some(end) = rest.find(')') {
                        let inner = &rest[..end];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() >= 3 {
                            let name = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let url = parts[1].trim().trim_matches('"').trim_matches('\'');
                            let rev = parts[2].trim().trim_matches('"').trim_matches('\'');
                            manifest.dependencies.push(PackageDependency {
                                name: name.to_string(),
                                git: Some(url.to_string()),
                                rev: Some(rev.to_string()),
                                ..Default::default()
                            });
                        }
                    }
                } else if trimmed.contains('"') {
                    // String literal dependency, e.g. "math_lib: 1.0.0" or "helpers: ./libs/helpers" or "net: git+https://..."
                    if let Some(first_quote) = trimmed.find('"') {
                        if let Some(second_quote) = trimmed[first_quote + 1..].find('"') {
                            let str_val = &trimmed[first_quote + 1..first_quote + 1 + second_quote];
                            if let Some((k, v)) =
                                str_val.split_once(':').or_else(|| str_val.split_once('='))
                            {
                                let name = k.trim().to_string();
                                let target = v.trim();
                                if target.starts_with("git+")
                                    || target.starts_with("https://")
                                    || target.starts_with("git@")
                                {
                                    manifest.dependencies.push(PackageDependency {
                                        name,
                                        git: Some(
                                            target
                                                .strip_prefix("git+")
                                                .unwrap_or(target)
                                                .to_string(),
                                        ),
                                        ..Default::default()
                                    });
                                } else if target.starts_with('.') || target.starts_with('/') {
                                    manifest.dependencies.push(PackageDependency {
                                        name,
                                        path: Some(target.to_string()),
                                        ..Default::default()
                                    });
                                } else {
                                    manifest.dependencies.push(PackageDependency {
                                        name,
                                        version: Some(target.to_string()),
                                        ..Default::default()
                                    });
                                }
                            } else {
                                manifest.dependencies.push(PackageDependency {
                                    name: str_val.trim().to_string(),
                                    version: Some("0.1.0".to_string()),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                continue;
            }

            // Check for block starts: optimization { ... }, build { ... }, network { ... }
            if trimmed.starts_with("optimization") && trimmed.contains('{') {
                current_block = "optimization";
                continue;
            }
            if trimmed.starts_with("build") && trimmed.contains('{') {
                current_block = "build";
                continue;
            }
            if trimmed.starts_with("network") && trimmed.contains('{') {
                current_block = "network";
                continue;
            }
            if trimmed == "}" {
                current_block = "root";
                continue;
            }

            // Parse key-value mappings
            if let Some((k, v)) = trimmed.split_once('=') {
                let k = k
                    .trim()
                    .trim_start_matches("string ")
                    .trim_start_matches("static ")
                    .trim_start_matches("const ")
                    .trim();
                let v = v
                    .trim()
                    .trim_matches(';')
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                match current_block {
                    "optimization" => match k {
                        "pgo" => manifest.optimization.pgo = Some(v.to_string()),
                        "telemetry_mode" => {
                            manifest.optimization.telemetry_mode = Some(v.to_string())
                        }
                        _ => {}
                    },
                    "build" => match k {
                        "mode" => manifest.build.mode = Some(v.to_string()),
                        "memory_domains" => manifest.build.memory_domains = Some(v.to_string()),
                        _ => {}
                    },
                    "network" => {
                        if k == "tls_provider" {
                            manifest.network.tls_provider = Some(v.to_string())
                        }
                    }
                    _ => match k {
                        "name" => manifest.name = v.to_string(),
                        "version" => manifest.version = v.to_string(),
                        "edition" => manifest.edition = v.to_string(),
                        "pgo" => manifest.optimization.pgo = Some(v.to_string()),
                        "telemetry_mode" => {
                            manifest.optimization.telemetry_mode = Some(v.to_string())
                        }
                        _ => {}
                    },
                }
            }
        }

        manifest
    }
}

pub fn find_package_manifest(start_dir: &Path) -> Option<PathBuf> {
    let mut current = if start_dir.is_file() {
        start_dir.parent()?.to_path_buf()
    } else {
        start_dir.to_path_buf()
    };
    loop {
        // Look for Package.syn first
        let syn_candidate = current.join("Package.syn");
        if syn_candidate.is_file() {
            return Some(syn_candidate);
        }
        // Fall back to syntra.toml
        let toml_candidate = current.join("syntra.toml");
        if toml_candidate.is_file() {
            return Some(toml_candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

pub fn find_syntra_toml(start_dir: &Path) -> Option<PathBuf> {
    find_package_manifest(start_dir)
}

/// Resolves, clones, or checks out a remote Git dependency.
/// Destination path: `<manifest_dir>/.syntra/packages/git/<safe_name>-<short_hash>`
pub fn resolve_git_dependency(
    manifest_dir: &Path,
    dep: &PackageDependency,
) -> Result<PathBuf, String> {
    let git_url = dep
        .git
        .as_ref()
        .ok_or_else(|| "Not a git dependency".to_string())?;
    let packages_git_dir = manifest_dir.join(".syntra").join("packages").join("git");

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    git_url.hash(&mut hasher);
    if let Some(b) = &dep.branch {
        b.hash(&mut hasher);
    }
    if let Some(t) = &dep.tag {
        t.hash(&mut hasher);
    }
    if let Some(r) = &dep.rev {
        r.hash(&mut hasher);
    }
    let short_hash = format!("{:08x}", hasher.finish());

    let safe_name = dep.name.replace(['/', ':'], "_");
    let dest_dir = packages_git_dir.join(format!("{safe_name}-{short_hash}"));

    if !dest_dir.exists() {
        fs::create_dir_all(&packages_git_dir)
            .map_err(|e| format!("Failed to create git packages dir: {e}"))?;

        let mut cmd = std::process::Command::new("git");
        cmd.arg("clone").arg("--quiet");

        if let Some(branch) = &dep.branch {
            cmd.arg("--branch").arg(branch);
        } else if let Some(tag) = &dep.tag {
            cmd.arg("--branch").arg(tag);
        }

        cmd.arg(git_url).arg(&dest_dir);

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to execute 'git clone': {e}"))?;
        if !status.success() {
            return Err(format!(
                "'git clone' exited with status {status} for {git_url}"
            ));
        }

        if let Some(rev) = &dep.rev {
            let checkout = std::process::Command::new("git")
                .current_dir(&dest_dir)
                .args(["checkout", "--quiet", rev])
                .status()
                .map_err(|e| format!("Failed to execute 'git checkout': {e}"))?;
            if !checkout.success() {
                return Err(format!("'git checkout' failed for rev '{rev}'"));
            }
        }
    }

    Ok(dest_dir)
}

/// Multi-file module loader and dependency resolver for Syntra.
pub struct ModuleLoader {
    loaded_files: HashSet<PathBuf>,
    search_paths: Vec<PathBuf>,
    pub sources: HashMap<PathBuf, String>,
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            loaded_files: HashSet::new(),
            search_paths: vec![
                PathBuf::from("."),
                PathBuf::from("stdlib"),
                PathBuf::from("packages"),
                PathBuf::from(".syntra/packages"),
            ],
            sources: HashMap::new(),
        }
    }

    pub fn with_search_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.search_paths.extend(paths);
        self
    }

    /// Loads an entry-point Syntra source file and recursively resolves all `import` statements.
    pub fn load_program(&mut self, root_path: &Path) -> Result<Program, Vec<Diagnostic>> {
        let canonical_root = root_path
            .canonicalize()
            .unwrap_or_else(|_| root_path.to_path_buf());
        self.loaded_files.insert(canonical_root.clone());

        // Check if syntra.toml exists and resolve dependencies into search paths
        if let Some(manifest_path) = find_syntra_toml(root_path) {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                let manifest = PackageManifest::parse(&content);
                let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
                for dep in &manifest.dependencies {
                    if let Some(p) = &dep.path {
                        let direct = manifest_dir.join(p);
                        self.search_paths.push(direct.clone());
                        self.search_paths.push(direct.join("src"));
                    }
                    if dep.git.is_some() {
                        if let Ok(git_path) = resolve_git_dependency(manifest_dir, dep) {
                            self.search_paths.push(git_path.clone());
                            self.search_paths.push(git_path.join("src"));
                        }
                    }
                    let pkg_dir = manifest_dir
                        .join(".syntra")
                        .join("packages")
                        .join(&dep.name);
                    self.search_paths.push(pkg_dir.clone());
                    self.search_paths.push(pkg_dir.join("src"));

                    let local_pkg_dir = manifest_dir.join("packages").join(&dep.name);
                    self.search_paths.push(local_pkg_dir.clone());
                    self.search_paths.push(local_pkg_dir.join("src"));
                }
            }
        }

        let source = fs::read_to_string(root_path).map_err(|err| {
            vec![Diagnostic::error(
                "LANG-001",
                format!(
                    "Failed to read source file '{}': {}",
                    root_path.display(),
                    err
                ),
                Span::default(),
            )]
        })?;

        self.sources.insert(canonical_root, source.clone());

        let root_prog = syntra_syntax::parse(&source)?;
        let mut final_items = Vec::new();

        for item in root_prog.items {
            if let Item::Import(import_decl) = &item {
                let imported_items =
                    self.resolve_import(&import_decl.path, root_path, import_decl.span)?;
                final_items.extend(imported_items);
            } else {
                final_items.push(item);
            }
        }

        Ok(Program { items: final_items })
    }

    fn resolve_import(
        &mut self,
        import_path: &str,
        current_file: &Path,
        import_span: Span,
    ) -> Result<Vec<Item>, Vec<Diagnostic>> {
        let resolved_file = self
            .find_module_file(import_path, current_file)
            .ok_or_else(|| {
                vec![Diagnostic::error(
                    "LANG-021",
                    format!(
                        "Could not resolve import '{}' from '{}'",
                        import_path,
                        current_file.display()
                    ),
                    import_span,
                )]
            })?;

        let canonical = resolved_file
            .canonicalize()
            .unwrap_or_else(|_| resolved_file.clone());
        if self.loaded_files.contains(&canonical) {
            // Already loaded (prevents duplicate compilation and circular import loops)
            return Ok(Vec::new());
        }
        self.loaded_files.insert(canonical.clone());

        let source = fs::read_to_string(&resolved_file).map_err(|err| {
            vec![Diagnostic::error(
                "LANG-001",
                format!(
                    "Failed to read imported file '{}': {}",
                    resolved_file.display(),
                    err
                ),
                import_span,
            )]
        })?;

        self.sources.insert(canonical, source.clone());

        let prog = syntra_syntax::parse(&source)?;

        let mut child_items = Vec::new();
        for item in prog.items {
            if let Item::Import(child_import) = &item {
                let nested =
                    self.resolve_import(&child_import.path, &resolved_file, child_import.span)?;
                child_items.extend(nested);
            } else {
                child_items.push(item);
            }
        }

        // Determine module name (e.g., from "utils.geometry" -> "geometry", "math.syn" -> "math")
        let clean_path = import_path.strip_suffix(".syn").unwrap_or(import_path);
        let mod_part = clean_path.rsplit('/').next().unwrap_or(clean_path);
        let module_name = mod_part.rsplit('.').next().unwrap_or(mod_part);

        let mut module_items = Vec::new();

        // 1. Check if the file already defines a namespace matching this module name
        let already_has_matching_namespace = child_items.iter().any(|item| {
            if let Item::Namespace(ns) = item {
                ns.name.eq_ignore_ascii_case(module_name)
            } else {
                false
            }
        });

        // 2. Separate top-level items
        let mut top_level_classes = Vec::new();
        let mut top_level_funcs = Vec::new();
        let mut other_items = Vec::new();

        for item in child_items {
            match item {
                Item::Class(_) | Item::Interface(_) => {
                    top_level_classes.push(item);
                }
                Item::Function(_) => {
                    top_level_funcs.push(item);
                }
                _ => {
                    other_items.push(item);
                }
            }
        }

        // Always make classes/interfaces and other namespaces available at top-level
        module_items.extend(top_level_classes.clone());
        module_items.extend(other_items);

        if already_has_matching_namespace {
            // If already in a namespace, keep functions as-is
            module_items.extend(top_level_funcs);
        } else {
            // Expose functions under module namespace: geometry.calculate_circle_area()
            if !top_level_funcs.is_empty() {
                module_items.push(Item::Namespace(NamespaceDecl {
                    name: module_name.to_string(),
                    doc: None,
                    items: top_level_funcs.clone(),
                    span: import_span,
                }));
                // Also expose at top level for convenience
                module_items.extend(top_level_funcs);
            }
        }

        Ok(module_items)
    }

    fn find_module_file(&self, import_path: &str, current_file: &Path) -> Option<PathBuf> {
        let parent_dir = current_file.parent().unwrap_or_else(|| Path::new("."));

        // 1. If path is a direct file path ending in an extension or containing slashes
        if import_path.ends_with(".syn")
            || import_path.ends_with(".sy")
            || import_path.ends_with(".ae")
            || import_path.contains('/')
            || import_path.contains('\\')
        {
            let direct = parent_dir.join(import_path);
            if direct.is_file() {
                return Some(direct);
            }
            for sp in &self.search_paths {
                let direct_sp = sp.join(import_path);
                if direct_sp.is_file() {
                    return Some(direct_sp);
                }
            }
        }

        let clean_path = if let Some(stripped) = import_path.strip_suffix(".syn") {
            stripped
        } else if let Some(stripped) = import_path.strip_suffix(".sy") {
            stripped
        } else if let Some(stripped) = import_path.strip_suffix(".ae") {
            stripped
        } else {
            import_path
        };

        // Convert dot notation to path: e.g. "utils.geometry" -> "utils/geometry"
        let rel_str = clean_path.replace('.', "/");
        let syn_name = format!("{rel_str}.syn");
        let syn_mod = format!("{rel_str}/mod.syn");
        let sy_name = format!("{rel_str}.sy");
        let sy_mod = format!("{rel_str}/mod.sy");
        let ae_name = format!("{rel_str}.ae");
        let ae_mod = format!("{rel_str}/mod.ae");

        let candidates = [
            parent_dir.join(&syn_name),
            parent_dir.join(&syn_mod),
            parent_dir.join(&sy_name),
            parent_dir.join(&sy_mod),
            parent_dir.join(&ae_name),
            parent_dir.join(&ae_mod),
        ];

        for c in &candidates {
            if c.is_file() {
                return Some(c.clone());
            }
        }

        // 2. Check search paths
        for sp in &self.search_paths {
            let sp_candidates = [
                sp.join(&syn_name),
                sp.join(&syn_mod),
                sp.join(&sy_name),
                sp.join(&sy_mod),
                sp.join(&ae_name),
                sp.join(&ae_mod),
            ];
            for c in &sp_candidates {
                if c.is_file() {
                    return Some(c.clone());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manifest_syntra_dsl() {
        let syn_manifest = r#"
        // Package.syn - Syntra Package Manifest
        package Config {
            name = "my_awesome_app";
            version = "1.2.3";
            edition = "2026";

            dependencies = [
                Dependency.version("math_lib", "2.0.0"),
                Dependency.path("helpers", "./libs/helpers"),
                Dependency.package("engine", "0.5.0", "./vendor/engine"),
                "network: 1.0.0",
                "storage = ./libs/storage"
            ];

            optimization {
                pgo = "locked";
                telemetry_mode = "sampling";
            }

            build {
                mode = "native";
                memory_domains = "strict";
            }

            network {
                tls_provider = "pure";
            }
        }
        "#;

        let manifest = PackageManifest::parse(syn_manifest);
        assert_eq!(manifest.name, "my_awesome_app");
        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.edition, "2026");
        assert_eq!(manifest.dependencies.len(), 5);
        assert_eq!(manifest.optimization.pgo.as_deref(), Some("locked"));
        assert_eq!(
            manifest.optimization.telemetry_mode.as_deref(),
            Some("sampling")
        );
        assert_eq!(manifest.build.mode.as_deref(), Some("native"));
        assert_eq!(manifest.build.memory_domains.as_deref(), Some("strict"));
        assert_eq!(manifest.network.tls_provider.as_deref(), Some("pure"));

        assert_eq!(manifest.dependencies[0].name, "math_lib");
        assert_eq!(manifest.dependencies[0].version.as_deref(), Some("2.0.0"));
        assert_eq!(manifest.dependencies[0].path, None);

        assert_eq!(manifest.dependencies[1].name, "helpers");
        assert_eq!(manifest.dependencies[1].version, None);
        assert_eq!(
            manifest.dependencies[1].path.as_deref(),
            Some("./libs/helpers")
        );

        assert_eq!(manifest.dependencies[2].name, "engine");
        assert_eq!(manifest.dependencies[2].version.as_deref(), Some("0.5.0"));
        assert_eq!(
            manifest.dependencies[2].path.as_deref(),
            Some("./vendor/engine")
        );

        assert_eq!(manifest.dependencies[3].name, "network");
        assert_eq!(manifest.dependencies[3].version.as_deref(), Some("1.0.0"));

        assert_eq!(manifest.dependencies[4].name, "storage");
        assert_eq!(
            manifest.dependencies[4].path.as_deref(),
            Some("./libs/storage")
        );
    }

    #[test]
    fn test_package_manifest_toml_backward_compat() {
        let toml_manifest = r#"
        [package]
        name = "legacy_pkg"
        version = "0.9.0"
        edition = "2026"

        [dependencies]
        math = "1.0.0"
        helpers = { path = "./helpers" }

        [optimization]
        pgo = "locked"
        telemetry_mode = "sampling"

        [build]
        mode = "native"

        [network]
        tls_provider = "pure"
        "#;

        let manifest = PackageManifest::parse(toml_manifest);
        assert_eq!(manifest.name, "legacy_pkg");
        assert_eq!(manifest.version, "0.9.0");
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.optimization.pgo.as_deref(), Some("locked"));
        assert_eq!(
            manifest.optimization.telemetry_mode.as_deref(),
            Some("sampling")
        );
        assert_eq!(manifest.build.mode.as_deref(), Some("native"));
        assert_eq!(manifest.network.tls_provider.as_deref(), Some("pure"));
    }

    #[test]
    fn test_package_manifest_git_dependencies() {
        let syn_manifest = r#"
        package Config {
            name = "git_app";
            version = "1.0.0";
            dependencies = [
                Dependency.git("remote_lib", "https://github.com/syntra-lang/remote-lib.git"),
                Dependency.gitBranch("feature_pkg", "https://github.com/syntra-lang/feature.git", "develop"),
                Dependency.gitTag("stable_pkg", "https://github.com/syntra-lang/stable.git", "v2.0.0"),
                Dependency.gitRev("pinned_pkg", "https://github.com/syntra-lang/pinned.git", "abc1234"),
                "simple_git: git+https://github.com/syntra-lang/simple.git"
            ];
        }
        "#;
        let manifest = PackageManifest::parse(syn_manifest);
        assert_eq!(manifest.dependencies.len(), 5);

        assert_eq!(manifest.dependencies[0].name, "remote_lib");
        assert_eq!(
            manifest.dependencies[0].git.as_deref(),
            Some("https://github.com/syntra-lang/remote-lib.git")
        );
        assert_eq!(manifest.dependencies[0].branch, None);

        assert_eq!(manifest.dependencies[1].name, "feature_pkg");
        assert_eq!(
            manifest.dependencies[1].git.as_deref(),
            Some("https://github.com/syntra-lang/feature.git")
        );
        assert_eq!(manifest.dependencies[1].branch.as_deref(), Some("develop"));

        assert_eq!(manifest.dependencies[2].name, "stable_pkg");
        assert_eq!(
            manifest.dependencies[2].git.as_deref(),
            Some("https://github.com/syntra-lang/stable.git")
        );
        assert_eq!(manifest.dependencies[2].tag.as_deref(), Some("v2.0.0"));

        assert_eq!(manifest.dependencies[3].name, "pinned_pkg");
        assert_eq!(
            manifest.dependencies[3].git.as_deref(),
            Some("https://github.com/syntra-lang/pinned.git")
        );
        assert_eq!(manifest.dependencies[3].rev.as_deref(), Some("abc1234"));

        assert_eq!(manifest.dependencies[4].name, "simple_git");
        assert_eq!(
            manifest.dependencies[4].git.as_deref(),
            Some("https://github.com/syntra-lang/simple.git")
        );

        let toml_manifest = r#"
        [package]
        name = "git_toml"
        version = "1.0.0"

        [dependencies]
        remote = { git = "https://github.com/syntra-lang/remote.git", branch = "main" }
        tagged = { git = "https://github.com/syntra-lang/tagged.git", tag = "v1.0" }
        direct_git = "git+https://github.com/syntra-lang/direct.git"
        "#;
        let toml_parsed = PackageManifest::parse(toml_manifest);
        assert_eq!(toml_parsed.dependencies.len(), 3);
        assert_eq!(toml_parsed.dependencies[0].name, "remote");
        assert_eq!(
            toml_parsed.dependencies[0].git.as_deref(),
            Some("https://github.com/syntra-lang/remote.git")
        );
        assert_eq!(toml_parsed.dependencies[0].branch.as_deref(), Some("main"));
        assert_eq!(toml_parsed.dependencies[1].name, "tagged");
        assert_eq!(toml_parsed.dependencies[1].tag.as_deref(), Some("v1.0"));
        assert_eq!(toml_parsed.dependencies[2].name, "direct_git");
        assert_eq!(
            toml_parsed.dependencies[2].git.as_deref(),
            Some("https://github.com/syntra-lang/direct.git")
        );
    }
}
