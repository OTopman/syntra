import * as vscode from 'vscode';

/**
 * Validates whether a given document is a Syntra Package Manifest (Package.syn / syntra.toml).
 */
export function isManifestDocument(document: vscode.TextDocument): boolean {
    const fileName = document.fileName.replace(/\\/g, '/');
    if (fileName.endsWith('Package.syn') || fileName.endsWith('syntra.toml')) {
        return true;
    }
    // Fallback: check first 5 lines for manifest signature
    const lineCount = Math.min(document.lineCount, 5);
    for (let i = 0; i < lineCount; i++) {
        const text = document.lineAt(i).text.trim();
        if (text.startsWith('// Package.syn') || text.startsWith('package Config') || text.startsWith('[package]')) {
            return true;
        }
    }
    return false;
}

export type ManifestSection = 'root' | 'package' | 'optimization' | 'build' | 'network' | 'dependencies';

/**
 * Determines the current section context within a Package.syn document.
 */
export function getManifestContext(document: vscode.TextDocument, position: vscode.Position): {
    section: ManifestSection;
    inQuotes: boolean;
    currentKey: string | null;
} {
    const textBefore = document.getText(new vscode.Range(new vscode.Position(0, 0), position));
    const currentLine = document.lineAt(position.line).text.substring(0, position.character);

    // Check quotes on current line
    const quotesCount = (currentLine.match(/"/g) || []).length;
    const inQuotes = quotesCount % 2 === 1;

    // Detect current key if on assignment line
    let currentKey: string | null = null;
    const assignMatch = currentLine.match(/([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*"?([^"]*)$/);
    if (assignMatch) {
        currentKey = assignMatch[1];
    }

    // Determine current block by inspecting braces/brackets
    const lines = textBefore.split('\n');
    let section: ManifestSection = 'package';
    let blockStack: string[] = [];

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        if (line.startsWith('//') || line.startsWith('#')) continue;

        if (line.includes('dependencies') && (line.includes('[') || line.includes('='))) {
            blockStack.push('dependencies');
            if (line.includes(']')) {
                blockStack.pop();
            }
        } else if (line.includes('optimization') && line.includes('{')) {
            blockStack.push('optimization');
        } else if (line.includes('build') && line.includes('{')) {
            blockStack.push('build');
        } else if (line.includes('network') && line.includes('{')) {
            blockStack.push('network');
        } else if (line.includes('package') && line.includes('{')) {
            blockStack.push('package');
        } else if (line.includes('}')) {
            blockStack.pop();
        } else if (line.includes(']')) {
            if (blockStack.length > 0 && blockStack[blockStack.length - 1] === 'dependencies') {
                blockStack.pop();
            }
        }
    }

    if (blockStack.length > 0) {
        const top = blockStack[blockStack.length - 1];
        if (top === 'dependencies') section = 'dependencies';
        else if (top === 'optimization') section = 'optimization';
        else if (top === 'build') section = 'build';
        else if (top === 'network') section = 'network';
        else if (top === 'package') section = 'package';
    }

    return { section, inQuotes, currentKey };
}

/**
 * Returns comprehensive completions for Package.syn.
 */
export function getManifestCompletions(
    document: vscode.TextDocument,
    position: vscode.Position,
    linePrefix: string
): vscode.CompletionItem[] {
    const items: vscode.CompletionItem[] = [];
    const ctx = getManifestContext(document, position);
    const trimmed = linePrefix.trim();

    // 1. Dependency method completions triggered on "Dependency."
    if (linePrefix.endsWith('Dependency.') || /Dependency\.\s*$/.test(linePrefix)) {
        return getDependencyStaticMethods();
    }

    // 2. Value completions after '=' (e.g. pgo = "|", mode = "|", memory_domains = "|")
    if (ctx.currentKey) {
        const valueItems = getValueCompletions(ctx.currentKey, ctx.inQuotes);
        if (valueItems.length > 0) {
            return valueItems;
        }
    }

    // 3. Section-specific completions
    switch (ctx.section) {
        case 'dependencies':
            items.push(...getDependenciesSectionCompletions());
            break;
        case 'optimization':
            items.push(...getOptimizationSectionCompletions());
            break;
        case 'build':
            items.push(...getBuildSectionCompletions());
            break;
        case 'network':
            items.push(...getNetworkSectionCompletions());
            break;
        case 'package':
        default:
            items.push(...getPackageSectionCompletions());
            break;
    }

    // Always include Dependency module if in dependencies array or if typing Dependency
    if (trimmed.startsWith('Dep') || ctx.section === 'dependencies') {
        items.push(createModuleItem('Dependency', 'Syntra package dependency constructor module.'));
        items.push(...getDependencyStaticMethods());
    }

    return items;
}

function getPackageSectionCompletions(): vscode.CompletionItem[] {
    const items: vscode.CompletionItem[] = [];

    // Fields
    items.push(createFieldSnippet(
        'name',
        'string',
        'Unique package identifier',
        'name = "${1:my_project}"'
    ));

    items.push(createFieldSnippet(
        'version',
        'string (semver)',
        'Package semantic version string',
        'version = "${1:0.1.0}"'
    ));

    items.push(createFieldSnippet(
        'edition',
        'string ("2026" | "2025")',
        'Syntra language edition specification',
        'edition = "${1|2026,2025|}"'
    ));

    // Section blocks
    items.push(createBlockSnippet(
        'optimization',
        'Optimization & Telemetry Configuration block',
        'optimization {\n\tpgo = "${1|locked,profile,adaptive,off|}";\n\ttelemetry_mode = "${2|sampling,continuous,off|}";\n}'
    ));

    items.push(createBlockSnippet(
        'build',
        'Compilation & Native Target Configuration block',
        'build {\n\tmode = "${1|native,interpreter,jit|}";\n\tmemory_domains = "${2|strict,permissive,arena|}";\n}'
    ));

    items.push(createBlockSnippet(
        'network',
        'Network & TLS Provider Configuration block',
        'network {\n\ttls_provider = "${1|pure,native,rustls,openssl|}";\n}'
    ));

    items.push(createBlockSnippet(
        'dependencies',
        'Package dependencies list',
        'dependencies = [\n\t$0\n]'
    ));

    // Full Package boilerplate snippet
    items.push(createBlockSnippet(
        'package Config',
        'Complete Syntra Package Manifest Boilerplate',
        'package Config {\n\tname = "${1:my_package}"\n\tversion = "${2:0.1.0}"\n\tedition = "2026"\n\n\toptimization {\n\t\tpgo = "locked"\n\t\ttelemetry_mode = "sampling"\n\t}\n\n\tbuild {\n\t\tmode = "native"\n\t\tmemory_domains = "strict"\n\t}\n\n\tnetwork {\n\t\ttls_provider = "pure"\n\t}\n\n\tdependencies = [\n\t\t$0\n\t]\n}'
    ));

    return items;
}

function getOptimizationSectionCompletions(): vscode.CompletionItem[] {
    return [
        createFieldSnippet(
            'pgo',
            '"locked" | "profile" | "adaptive" | "off"',
            'Profile-Guided Optimization strategy. "locked" freezes profile.synlock; "profile" samples hot traces; "adaptive" adjusts thresholds online.',
            'pgo = "${1|locked,profile,adaptive,off|}"'
        ),
        createFieldSnippet(
            'telemetry_mode',
            '"sampling" | "continuous" | "off"',
            'Telemetry collection mode for the Syntra Adaptive Runtime (SAR) and machine-learning governor.',
            'telemetry_mode = "${1|sampling,continuous,off|}"'
        ),
    ];
}

function getBuildSectionCompletions(): vscode.CompletionItem[] {
    return [
        createFieldSnippet(
            'mode',
            '"native" | "interpreter" | "jit"',
            'Default compilation target mode. "native" generates standalone machine code via Cranelift; "interpreter" runs bytecode AST.',
            'mode = "${1|native,interpreter,jit|}"'
        ),
        createFieldSnippet(
            'memory_domains',
            '"strict" | "permissive" | "arena"',
            'Memory domain isolation and ownership safety level. "strict" enforces zero-leak linear ownership semantics (MEM-001..004).',
            'memory_domains = "${1|strict,permissive,arena|}"'
        ),
    ];
}

function getNetworkSectionCompletions(): vscode.CompletionItem[] {
    return [
        createFieldSnippet(
            'tls_provider',
            '"pure" | "native" | "rustls" | "openssl"',
            'Cryptographic TLS engine provider for network sockets, HTTP client/server, and WebSockets.',
            'tls_provider = "${1|pure,native,rustls,openssl|}"'
        ),
    ];
}

function getDependenciesSectionCompletions(): vscode.CompletionItem[] {
    return [
        ...getDependencyStaticMethods(),
        createSnippetItem(
            'Dependency.version',
            'Add versioned package dependency from registry',
            'Dependency.version("${1:name}", "${2:0.1.0}"),'
        ),
        createSnippetItem(
            'Dependency.path',
            'Add local filesystem path dependency',
            'Dependency.path("${1:name}", "${2:./libs/name}"),'
        ),
        createSnippetItem(
            'Dependency.package',
            'Add dependency with version and local fallback path',
            'Dependency.package("${1:name}", "${2:0.1.0}", "${3:./libs/name}"),'
        ),
        createSnippetItem(
            'Dependency.git',
            'Add remote Git repository dependency',
            'Dependency.git("${1:name}", "${2:https://github.com/user/repo}"),'
        ),
        createSnippetItem(
            'Dependency.gitBranch',
            'Add Git dependency pinned to specific branch',
            'Dependency.gitBranch("${1:name}", "${2:https://github.com/user/repo}", "${3:main}"),'
        ),
        createSnippetItem(
            'Dependency.gitTag',
            'Add Git dependency pinned to specific release tag',
            'Dependency.gitTag("${1:name}", "${2:https://github.com/user/repo}", "${3:v1.0.0}"),'
        ),
        createSnippetItem(
            'Dependency.gitRev',
            'Add Git dependency pinned to exact commit SHA hash',
            'Dependency.gitRev("${1:name}", "${2:https://github.com/user/repo}", "${3:commit_sha}"),'
        ),
        createSnippetItem(
            '"name: version"',
            'Compact string dependency definition',
            '"${1:package_name}: ${2:0.1.0}",'
        ),
        createSnippetItem(
            '"name: path"',
            'Compact string path dependency definition',
            '"${1:package_name}: ${2:./path}",'
        ),
        createSnippetItem(
            '"name: git"',
            'Compact string Git dependency definition',
            '"${1:package_name}: git+${2:https://github.com/user/repo}",'
        ),
    ];
}

export function getDependencyStaticMethods(): vscode.CompletionItem[] {
    return [
        createMethodItem(
            'version(name, version)',
            'Dependency',
            'Declares a registry dependency with semantic version constraints.',
            'version("${1:name}", "${2:0.1.0}")'
        ),
        createMethodItem(
            'path(name, localPath)',
            'Dependency',
            'Declares a local filesystem directory dependency.',
            'path("${1:name}", "${2:./path}")'
        ),
        createMethodItem(
            'package(name, version, localPath)',
            'Dependency',
            'Declares a dependency with semantic version and local path resolution.',
            'package("${1:name}", "${2:0.1.0}", "${3:./path}")'
        ),
        createMethodItem(
            'git(name, repositoryUrl)',
            'Dependency',
            'Declares a Git repository dependency fetched over HTTPS or SSH.',
            'git("${1:name}", "${2:https://github.com/user/repo}")'
        ),
        createMethodItem(
            'gitBranch(name, repositoryUrl, branch)',
            'Dependency',
            'Declares a Git repository dependency pinned to a target branch.',
            'gitBranch("${1:name}", "${2:https://github.com/user/repo}", "${3:main}")'
        ),
        createMethodItem(
            'gitTag(name, repositoryUrl, tag)',
            'Dependency',
            'Declares a Git repository dependency pinned to a target release tag.',
            'gitTag("${1:name}", "${2:https://github.com/user/repo}", "${3:v1.0.0}")'
        ),
        createMethodItem(
            'gitRev(name, repositoryUrl, rev)',
            'Dependency',
            'Declares a Git repository dependency pinned to a cryptographic commit SHA hash.',
            'gitRev("${1:name}", "${2:https://github.com/user/repo}", "${3:commit_sha}")'
        ),
    ];
}

function getValueCompletions(key: string, inQuotes: boolean): vscode.CompletionItem[] {
    const wrap = (val: string) => inQuotes ? val : `"${val}"`;
    switch (key) {
        case 'pgo':
            return [
                createValueItem('locked', wrap('locked'), 'Profile-Guided Optimization is locked to validated profile.synlock baseline.'),
                createValueItem('profile', wrap('profile'), 'Profile mode actively collects hot-path execution frequency profiles.'),
                createValueItem('adaptive', wrap('adaptive'), 'Adaptive PGO continuously adjusts compilation tiers dynamically.'),
                createValueItem('off', wrap('off'), 'PGO is completely disabled; standard baseline optimization applies.'),
            ];
        case 'telemetry_mode':
            return [
                createValueItem('sampling', wrap('sampling'), 'Samples performance telemetry at low overhead (default 5%).'),
                createValueItem('continuous', wrap('continuous'), 'Continuously records all execution metrics for comprehensive intelligence analysis.'),
                createValueItem('off', wrap('off'), 'Disables telemetry recording.'),
            ];
        case 'mode':
            return [
                createValueItem('native', wrap('native'), 'Compile to standalone native machine binary (Mach-O on macOS, ELF on Linux).'),
                createValueItem('interpreter', wrap('interpreter'), 'Execute via AST bytecode tree-walk interpreter.'),
                createValueItem('jit', wrap('jit'), 'Execute with Just-In-Time native tier compilation.'),
            ];
        case 'memory_domains':
            return [
                createValueItem('strict', wrap('strict'), 'Strict zero-leak linear ownership and memory domain boundary verification (MEM-001..004).'),
                createValueItem('permissive', wrap('permissive'), 'Permissive domain boundary checks with runtime fallback assertions.'),
                createValueItem('arena', wrap('arena'), 'Fast arena-allocated memory domain with bulk region deallocation.'),
            ];
        case 'tls_provider':
            return [
                createValueItem('pure', wrap('pure'), 'Pure memory-safe cryptographic TLS 1.3 implementation.'),
                createValueItem('native', wrap('native'), 'Operating system native TLS stack (Security.framework / schannel).'),
                createValueItem('rustls', wrap('rustls'), 'Modern memory-safe Rustls TLS library.'),
                createValueItem('openssl', wrap('openssl'), 'Standard OpenSSL / LibreSSL crypto backend.'),
            ];
        case 'edition':
            return [
                createValueItem('2026', wrap('2026'), 'Syntra 2026 Edition: AIR invariants, SAR adaptive runtime, strict memory domains.'),
                createValueItem('2025', wrap('2025'), 'Syntra 2025 Edition: Baseline language specification.'),
            ];
        default:
            return [];
    }
}

/**
 * Rich Hover documentation lookup for manifest symbols.
 */
export function getManifestHoverInfo(word: string, lineText: string): string | null {
    if (word === 'Dependency' || lineText.includes('Dependency.')) {
        const methodMatch = lineText.match(/Dependency\.([a-zA-Z0-9_]+)/);
        if (methodMatch) {
            const method = methodMatch[1];
            if (MANIFEST_HOVER_DICTIONARY[`Dependency.${method}`]) {
                return MANIFEST_HOVER_DICTIONARY[`Dependency.${method}`];
            }
        }
    }

    return MANIFEST_HOVER_DICTIONARY[word] || null;
}

const MANIFEST_HOVER_DICTIONARY: Record<string, string> = {
    'package': '```syntra\npackage Config { ... }\n```\nDeclares the primary package manifest container. Encapsulates package metadata, optimization flags, build settings, and dependencies.',
    'Config': 'Root configuration container block for `Package.syn`.',
    'name': '```syntra\nname = "package_name"\n```\nUnique identifier for this package. Used for imports, binary artifact naming, and package registry publication.',
    'version': '```syntra\nversion = "x.y.z"\n```\nSemantic versioning string (SemVer 2.0.0) describing the package release state.',
    'edition': '```syntra\nedition = "2026"\n```\nSpecifies the Syntra compiler edition. `"2026"` enables strict ownership domains, AIR invariant enforcement, and the Self-Adapting Runtime (SAR).',
    'optimization': '```syntra\noptimization {\n    pgo = "locked"\n    telemetry_mode = "sampling"\n}\n```\nControls Profile-Guided Optimization (PGO) and runtime telemetry sampling policies.',
    'pgo': '```syntra\npgo = "locked" | "profile" | "adaptive" | "off"\n```\nProfile-Guided Optimization state.\n* `"locked"`: Re-uses frozen `profile.synlock` profiles for deterministic optimization.\n* `"profile"`: Collects and records branch/call execution counts to update profiles.\n* `"adaptive"`: Real-time dynamic tiering with SAR.\n* `"off"`: Standard baseline compilation without PGO.',
    'telemetry_mode': '```syntra\ntelemetry_mode = "sampling" | "continuous" | "off"\n```\nConfigures execution profiling sampling frequency for learning and optimization pipelines.',
    'build': '```syntra\nbuild {\n    mode = "native"\n    memory_domains = "strict"\n}\n```\nConfigures compilation backend targets and memory ownership checking rules.',
    'mode': '```syntra\nmode = "native" | "interpreter" | "jit"\n```\nCompilation target format.\n* `"native"`: Generates fast standalone machine binaries via Cranelift.\n* `"interpreter"`: High-speed AST bytecode execution.\n* `"jit"`: Just-In-Time machine compilation.',
    'memory_domains': '```syntra\nmemory_domains = "strict" | "permissive" | "arena"\n```\nEnforces memory ownership verification (MEM-001..004).\n* `"strict"`: Verifies compile-time exclusive borrow and domain isolation.\n* `"permissive"`: Allows dynamic domain crossing with assertions.\n* `"arena"`: Region-based allocation and reclamation.',
    'network': '```syntra\nnetwork {\n    tls_provider = "pure"\n}\n```\nNetwork protocol and cryptography subsystem configuration.',
    'tls_provider': '```syntra\ntls_provider = "pure" | "native" | "rustls" | "openssl"\n```\nTLS 1.3 encryption provider used for HTTPS, WSS, and secure socket connections.',
    'dependencies': '```syntra\ndependencies = [\n    Dependency.version("math", "1.0.0"),\n    Dependency.path("utils", "./libs/utils"),\n    Dependency.git("net", "https://github.com/syntra-lang/net")\n]\n```\nList of external package dependencies resolved and locked in `Package.lock`.',
    'Dependency': '```syntra\nmodule Dependency\n```\nConstructor API for declaring package dependencies with version constraints, paths, or Git repositories.',
    'Dependency.version': '```syntra\nDependency.version(name: string, version: string): Dependency\n```\nDeclares a dependency resolved from the official package registry matching semantic version constraints.',
    'Dependency.path': '```syntra\nDependency.path(name: string, localPath: string): Dependency\n```\nDeclares a dependency sourced from a local directory relative to `Package.syn`.',
    'Dependency.package': '```syntra\nDependency.package(name: string, version: string, localPath: string): Dependency\n```\nDeclares a dependency with a version tag and a local directory fallback path.',
    'Dependency.git': '```syntra\nDependency.git(name: string, url: string): Dependency\n```\nDeclares a remote Git dependency cloned over HTTPS or SSH.',
    'Dependency.gitBranch': '```syntra\nDependency.gitBranch(name: string, url: string, branch: string): Dependency\n```\nDeclares a Git dependency pinned to a specific branch name.',
    'Dependency.gitTag': '```syntra\nDependency.gitTag(name: string, url: string, tag: string): Dependency\n```\nDeclares a Git dependency pinned to a release tag (e.g. `"v1.2.0"`).',
    'Dependency.gitRev': '```syntra\nDependency.gitRev(name: string, url: string, rev: string): Dependency\n```\nDeclares a Git dependency pinned to an immutable commit SHA hash.',
};

/**
 * Signature Information lookup for Dependency.* calls.
 */
export function getManifestSignatureInfo(callee: string): vscode.SignatureInformation | null {
    return MANIFEST_SIGNATURES[callee] || null;
}

const MANIFEST_SIGNATURES: Record<string, vscode.SignatureInformation> = {
    'Dependency.version': createSig(
        'Dependency.version(name: string, version: string): Dependency',
        'Declares a package dependency with semantic version constraint.',
        [
            { label: 'name: string', doc: 'Package identifier (e.g. "syntra_math")' },
            { label: 'version: string', doc: 'Semantic version requirement (e.g. "0.1.0" or "^1.2")' },
        ]
    ),
    'Dependency.path': createSig(
        'Dependency.path(name: string, localPath: string): Dependency',
        'Declares a local filesystem directory dependency.',
        [
            { label: 'name: string', doc: 'Package identifier' },
            { label: 'localPath: string', doc: 'Relative path to dependency root (e.g. "./libs/common")' },
        ]
    ),
    'Dependency.package': createSig(
        'Dependency.package(name: string, version: string, localPath: string): Dependency',
        'Declares a dependency with semantic version and local path resolution.',
        [
            { label: 'name: string', doc: 'Package identifier' },
            { label: 'version: string', doc: 'Semantic version tag' },
            { label: 'localPath: string', doc: 'Local directory fallback path' },
        ]
    ),
    'Dependency.git': createSig(
        'Dependency.git(name: string, repositoryUrl: string): Dependency',
        'Declares a Git repository dependency.',
        [
            { label: 'name: string', doc: 'Package identifier' },
            { label: 'repositoryUrl: string', doc: 'Remote Git URL (e.g. "https://github.com/user/repo")' },
        ]
    ),
    'Dependency.gitBranch': createSig(
        'Dependency.gitBranch(name: string, repositoryUrl: string, branch: string): Dependency',
        'Declares a Git repository dependency pinned to a branch.',
        [
            { label: 'name: string', doc: 'Package identifier' },
            { label: 'repositoryUrl: string', doc: 'Remote Git URL' },
            { label: 'branch: string', doc: 'Target Git branch name (e.g. "main" or "dev")' },
        ]
    ),
    'Dependency.gitTag': createSig(
        'Dependency.gitTag(name: string, repositoryUrl: string, tag: string): Dependency',
        'Declares a Git repository dependency pinned to a tag.',
        [
            { label: 'name: string', doc: 'Package identifier' },
            { label: 'repositoryUrl: string', doc: 'Remote Git URL' },
            { label: 'tag: string', doc: 'Target release tag (e.g. "v1.0.0")' },
        ]
    ),
    'Dependency.gitRev': createSig(
        'Dependency.gitRev(name: string, repositoryUrl: string, rev: string): Dependency',
        'Declares a Git repository dependency pinned to an immutable commit hash.',
        [
            { label: 'name: string', doc: 'Package identifier' },
            { label: 'repositoryUrl: string', doc: 'Remote Git URL' },
            { label: 'rev: string', doc: 'Full or abbreviated commit SHA hash' },
        ]
    ),
};

/**
 * Outline Document Symbols generator for Package.syn.
 */
export function getManifestSymbols(document: vscode.TextDocument): vscode.DocumentSymbol[] {
    const symbols: vscode.DocumentSymbol[] = [];
    const text = document.getText();
    const lines = text.split('\n');

    let packageSymbol: vscode.DocumentSymbol | null = null;
    let currentBlock: vscode.DocumentSymbol | null = null;

    for (let l = 0; l < lines.length; l++) {
        const line = lines[l];
        const trimmed = line.trim();
        if (trimmed.startsWith('//') || trimmed.startsWith('#')) continue;

        // package Config
        const pkgMatch = trimmed.match(/^package\s+([a-zA-Z0-9_]+)/);
        if (pkgMatch) {
            const name = `package ${pkgMatch[1]}`;
            const range = new vscode.Range(l, 0, lines.length - 1, 0);
            const selRange = new vscode.Range(l, 0, l, line.length);
            packageSymbol = new vscode.DocumentSymbol(name, 'Package Manifest', vscode.SymbolKind.Package, range, selRange);
            symbols.push(packageSymbol);
            continue;
        }

        // Sub-blocks: optimization, build, network, dependencies
        const blockMatch = trimmed.match(/^([a-zA-Z0-9_]+)\s*(\{|=\s*\[)/);
        if (blockMatch) {
            const blockName = blockMatch[1];
            if (['optimization', 'build', 'network', 'dependencies'].includes(blockName)) {
                const range = new vscode.Range(l, 0, l + 5, 0);
                const selRange = new vscode.Range(l, 0, l, line.length);
                currentBlock = new vscode.DocumentSymbol(blockName, 'Configuration Block', vscode.SymbolKind.Namespace, range, selRange);
                if (packageSymbol) {
                    packageSymbol.children.push(currentBlock);
                } else {
                    symbols.push(currentBlock);
                }
                continue;
            }
        }

        // Dependency entries inside dependencies array
        if (currentBlock && currentBlock.name === 'dependencies' && (trimmed.includes('Dependency.') || trimmed.startsWith('"'))) {
            const depMatch = trimmed.match(/Dependency\.[a-zA-Z0-9_]+\(\s*"([^"]+)"/) || trimmed.match(/"([^":]+)[:=]/);
            if (depMatch) {
                const depName = depMatch[1];
                const range = new vscode.Range(l, 0, l, line.length);
                const selRange = new vscode.Range(l, line.indexOf(depName), l, line.indexOf(depName) + depName.length);
                const depSym = new vscode.DocumentSymbol(depName, 'Dependency', vscode.SymbolKind.Module, range, selRange);
                currentBlock.children.push(depSym);
                continue;
            }
        }

        // Properties: name, version, edition, pgo, etc.
        const propMatch = trimmed.match(/^([a-zA-Z0-9_]+)\s*=\s*(.*)$/);
        if (propMatch && !trimmed.includes('[')) {
            const key = propMatch[1];
            const val = propMatch[2].replace(/[",;]/g, '').trim();
            const range = new vscode.Range(l, 0, l, line.length);
            const selRange = new vscode.Range(l, line.indexOf(key), l, line.indexOf(key) + key.length);
            const propSym = new vscode.DocumentSymbol(key, val, vscode.SymbolKind.Property, range, selRange);
            if (currentBlock) {
                currentBlock.children.push(propSym);
            } else if (packageSymbol) {
                packageSymbol.children.push(propSym);
            } else {
                symbols.push(propSym);
            }
        }

        if (trimmed === '}' || trimmed === ']' || trimmed === '];') {
            currentBlock = null;
        }
    }

    return symbols;
}

// Helpers
function createFieldSnippet(name: string, type: string, doc: string, snippet: string): vscode.CompletionItem {
    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Property);
    item.detail = `(manifest property) ${name}: ${type}`;
    item.documentation = new vscode.MarkdownString(doc);
    item.insertText = new vscode.SnippetString(snippet);
    return item;
}

function createBlockSnippet(name: string, doc: string, snippet: string): vscode.CompletionItem {
    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Class);
    item.detail = `(manifest block) ${name}`;
    item.documentation = new vscode.MarkdownString(doc);
    item.insertText = new vscode.SnippetString(snippet);
    return item;
}

function createSnippetItem(label: string, doc: string, snippet: string): vscode.CompletionItem {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
    item.detail = `(manifest dependency) ${label}`;
    item.documentation = new vscode.MarkdownString(doc);
    item.insertText = new vscode.SnippetString(snippet);
    return item;
}

function createValueItem(label: string, insertText: string, doc: string): vscode.CompletionItem {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.EnumMember);
    item.detail = `(value) "${label}"`;
    item.documentation = new vscode.MarkdownString(doc);
    item.insertText = insertText;
    return item;
}

function createModuleItem(name: string, doc: string): vscode.CompletionItem {
    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Module);
    item.detail = `module ${name}`;
    item.documentation = new vscode.MarkdownString(doc);
    return item;
}

function createMethodItem(label: string, returnType: string, doc: string, snippet: string): vscode.CompletionItem {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Method);
    item.detail = `(method) Dependency.${label}: ${returnType}`;
    item.documentation = new vscode.MarkdownString(doc);
    item.insertText = new vscode.SnippetString(snippet);
    return item;
}

function createSig(label: string, doc: string, params: Array<{ label: string; doc: string }>): vscode.SignatureInformation {
    const sig = new vscode.SignatureInformation(label, new vscode.MarkdownString(doc));
    sig.parameters = params.map(p => new vscode.ParameterInformation(p.label, new vscode.MarkdownString(p.doc)));
    return sig;
}
