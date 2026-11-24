# Native Cranelift Backend

The native compilation backend (`crates/backend_cranelift/`) compiles Syntra Intermediate Representation (AIR) into stripped, standalone native machine executables using the Cranelift code generator.

---

## ⚡ Architecture & Pipeline

```
            ┌────────────────────────────────────────┐
            │          Lowered AIR Module            │
            └───────────────────┬────────────────────┘
                                │
                   [Cranelift Context & ISA Builder]
                   (Target: ARM64 Apple Silicon / x86_64)
                                │
                   [Cranelift IR Translation]
                   (Instructions, Blocks, Call Conventions)
                                │
                   [Machine Code Emission]
                   (Direct Mach-O / ELF Code Buffer)
                                │
                   [Object File Linker & Symbol Table]
                   (External libc, Math, Allocator Resolvers)
                                │
                   ┌────────────┴────────────┐
                   ▼                         ▼
         [Mach-O Executable]          [ELF Executable]
          (macOS aarch64)              (Linux x86_64)
```

---

## 🎯 Target Platform Support

* **macOS (Apple Silicon ARM64):** Emits Mach-O 64-bit binaries targeting `aarch64-apple-darwin` with Apple ABI calling conventions.
* **Linux (x86_64 / aarch64):** Emits ELF 64-bit binaries conforming to the System V AMD64 ABI.
* **Windows (x86_64):** Targets Microsoft x64 calling conventions and PE/COFF formatting.

---

## 🚀 Native Compilation via CLI

```bash
syn build src/main.syn -o my_binary
```

### Features
* **Zero Runtime Overhead:** Compiled native binaries contain machine instructions with no interpreter or bytecode overhead.
* **Standalone Deployment:** Binaries run independently without requiring Rust, Cargo, or the `syn` toolchain to be present on the host system.
* **Calling Convention Compliance:** Foreign C functions can be linked directly using standard C ABI conventions.
