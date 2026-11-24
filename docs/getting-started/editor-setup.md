# Editor Setup (VS Code & Cursor)

Syntra provides a first-class editor extension written in 100% TypeScript with full IntelliSense, syntax autocomplete, parameter hints, hover documentation, breadcrumbs, and package manifest intelligence.

---

## 📦 Automated Installation

From the root of the Syntra repository, run the installation script:

```bash
./install_extension.sh
```

This compiles the TypeScript extension via `tsc -p ./` and installs the extension into:
* **VS Code:** `~/.vscode/extensions/syntra-lang`
* **Cursor:** `~/.cursor/extensions/syntra-lang`

### Reloading Your Editor
After running `./install_extension.sh`:
1. Open VS Code or Cursor.
2. Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux/Windows).
3. Type: `Developer: Reload Window` and press Enter.
4. Open any `.syn` file (such as `Package.syn` or `examples/hello.syn`).

---

## ✨ Extension Features

### 1. Smart Autocomplete & Member Access
* **Module Completions:** Typing `System.`, `Math.`, `File.`, `Json.`, `Task.`, `Channel.`, `ThreadPool.`, or `Dependency.` surfaces all available functions with parameter hints and return types.
* **Collection Methods:** Typing `.` on any array, map, or string variable surfaces methods like `push()`, `pop()`, `get()`, `set()`, `has()`, `contains()`, `keys()`, `values()`, `substring()`, `trim()`, and `split()`.
* **Type Annotations:** Typing `:` suggests all built-in primitive and generic types (`int`, `float`, `bool`, `string`, `void`, `Array<T>`, `Map<K, V>`, `Task<T>`, `Channel<T>`).

### 2. Package Manifest IntelliSense (`Package.syn`)
* When editing `Package.syn`, typing `=` or `"` immediately offers autocompletions for all configuration flags:
  * `pgo = ` -> `"locked"`, `"profile"`, `"adaptive"`, `"off"`
  * `mode = ` -> `"native"`, `"interpreter"`, `"jit"`
  * `memory_domains = ` -> `"strict"`, `"permissive"`, `"arena"`
  * `telemetry_mode = ` -> `"sampling"`, `"continuous"`, `"off"`
  * `tls_provider = ` -> `"pure"`, `"native"`, `"rustls"`, `"openssl"`
* In `dependencies = [`, typing `Dependency.` shows constructors for `version()`, `path()`, `package()`, `git()`, `gitBranch()`, `gitTag()`, and `gitRev()`.

### 3. Rich Markdown Hover Documentation
* Hovering over any keyword, standard library function, or manifest property shows detailed Markdown cards containing type signatures, specifications, and code snippets.

### 4. Live Parameter Hints (Signature Help)
* Typing `(` or `,` displays parameter tooltips with the currently active argument highlighted in bold.

### 5. Document Symbols & Outline Navigation
* The VS Code Outline Pane and top Breadcrumbs display nested classes, interfaces, methods, functions, and manifest blocks.

### 6. Interactive Status Bar
* The status bar displays `$(zap) Syntra (LSP: Ready)` or `$(circle-slash) Syntra (LSP: Offline)`.
* Clicking the status bar pill opens the **Syntra Quick Actions Menu** with one-click shortcuts to Run, Build, Check, Format, and Plan.

---

## ⚙️ Configuration Settings

Customize the extension in your `settings.json`:

```json
{
  "syntra.lsp.enabled": true,
  "syntra.lsp.path": "syn",
  "syntra.format.onSave": true,
  "syntra.sar.enabled": true,
  "syntra.intellisense.clientSideCompletions": true
}
```

Next, explore the [**Language Reference**](../language/index.md) to master Syntra's syntax and idioms.
