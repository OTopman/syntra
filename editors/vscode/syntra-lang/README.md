# Syntra Language Extension for VS Code (`syntra-lang`)

Official lightweight VS Code extension for the **Syntra** programming language.

## Features

- **Full Syntax Highlighting**: TextMate grammar for keywords (`fn`, `class`, `Task`, `Channel`, `ThreadPool`, `select`), strings with string interpolation (`\{expr}`), doc-comments (`///`), and types.
- **Interactive Language Server Protocol (LSP)**: Direct zero-dependency connection to `syn lsp`:
  - **Live Diagnostics**: Real-time syntax and type error squiggles as you type.
  - **Autocomplete & IntelliSense**: Keywords, built-in standard library classes, methods, and functions.
  - **Hover Documentation**: Detailed type signatures and markdown doc-comments on hover.
  - **Go to Definition**: Jump directly to functions, classes, and variable declarations (`F12`).
  - **Document Outline / Symbols**: File outline of classes, functions, and methods (`Ctrl+Shift+O` / `Cmd+Shift+O`).

---

## Installation

### Method 1: Automated Script (Fastest & Recommended)
Run the automated installation script from the project root:

```bash
./install_extension.sh
```

Then in VS Code, press `Cmd+Shift+P` (or `Ctrl+Shift+P`) and run **`Developer: Reload Window`**.

---

### Method 2: Manual Copy (Works from any directory)

Copy the extension folder directly into your VS Code extensions folder:

```bash
# macOS / Linux
mkdir -p ~/.vscode/extensions/syntra-lang && cp -R /Users/Topman/Documents/Projects/aether/editors/vscode/syntra-lang/* ~/.vscode/extensions/syntra-lang/

# Windows (Command Prompt)
xcopy /E /I "editors\vscode\syntra-lang" "%USERPROFILE%\.vscode\extensions\syntra-lang"
```

Then restart or reload VS Code:
1. Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Windows/Linux).
2. Select **`Developer: Reload Window`**.

---

### Method 3: Symlink

If you prefer symlinking for live development:

```bash
ln -sfn /Users/Topman/Documents/Projects/aether/editors/vscode/syntra-lang ~/.vscode/extensions/syntra-lang
```

---

## Configuration & Automatic Binary Detection

The extension automatically looks for the `syn` executable in:
1. `${workspaceFolder}/target/debug/syn` (automatic when working inside this repository!)
2. `${workspaceFolder}/target/release/syn`
3. `~/.cargo/bin/syn`
4. Standard system `PATH`

If you have installed `syn` in a custom directory, you can specify its path in your VS Code `settings.json`:

```json
{
  "syntra.lsp.path": "/Users/Topman/Documents/Projects/aether/target/debug/syn",
  "syntra.lsp.enabled": true
}
```
