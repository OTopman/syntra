# Installation Guide

This guide covers building and installing the Syntra compiler and runtime toolchain (`syn`) on your machine.

---

## 💻 System Requirements

Syntra is engineered for cross-platform portability. Supported operating systems and architectures include:

* **macOS:** Apple Silicon (ARM64) and Intel (x86_64) — macOS 12+
* **Linux:** Ubuntu, Debian, Fedora, Arch Linux, Alpine (x86_64 and aarch64) — glibc 2.28+ or musl
* **Windows:** Windows 10/11 (x86_64)

### Prerequisites

To compile the Syntra toolchain from source, ensure you have:
* **Rust & Cargo:** Version 1.75.0 or newer. Install via [rustup.rs](https://rustup.rs):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
* **Git:** Version 2.30+
* **C Compiler / Linker:** `clang` or `gcc` (macOS Xcode Command Line Tools: `xcode-select --install`)

---

## 🔨 Building the Toolchain from Source

1. Clone the Syntra repository:
   ```bash
   git clone https://github.com/syntra-lang/syntra.git
   cd syntra
   ```

2. Build the unified driver binary (`syn`):
   ```bash
   cargo build --release --bin syn
   ```

   The resulting executable will be placed in `target/release/syn`.

3. Verify the build:
   ```bash
   ./target/release/syn --version
   # Expected output: syntra 0.1.0 (syntra-lang 2026 edition)
   ```

---

## 🌐 Adding `syn` to Your System PATH

To invoke `syn` from any terminal session without specifying its absolute path, install it to your user binary directory:

### On macOS / Linux:
```bash
# Option A: Copy to Cargo bin directory (if ~/.cargo/bin is in your PATH)
cp target/release/syn ~/.cargo/bin/

# Option B: Symlink to /usr/local/bin (requires administrator privileges)
sudo ln -sf "$(pwd)/target/release/syn" /usr/local/bin/syn
```

Confirm that the binary is discoverable:
```bash
which syn
# Output: /Users/<user>/.cargo/bin/syn
syn --help
```

### On Windows:
```powershell
# Copy to Cargo bin folder
Copy-Item target\release\syn.exe "$env:USERPROFILE\.cargo\bin\syn.exe"
```

---

## 🧪 Validating Your Installation

Execute the test suite to ensure all 11 workspace crates operate cleanly on your system:

```bash
cargo test --workspace
```

All 59 tests should pass, confirming that the parser, type-checker, intermediate representation (AIR), runtime engine, Cranelift native backend, machine learning system (SLS), and Self-Adapting Runtime (SAR) are fully functional.

Next, continue to the [**Quickstart Tutorial**](quickstart.md) to create your first Syntra project.
