# The Syntra Programming Language

[![Workspace Tests](https://img.shields.io/badge/tests-115%2F115%20passing-brightgreen.svg)](crates/)
[![Language Edition](https://img.shields.io/badge/edition-2026-blue.svg)](specifications/01-master-language-specification.md)
[![License](https://img.shields.io/badge/license-MIT-purple.svg)](Cargo.toml)
[![VS Code Extension](https://img.shields.io/badge/vscode-TypeScript%20IntelliSense-blueviolet.svg)](editors/vscode/syntra-lang/)

**Syntra** is a next-generation, high-performance systems and general-purpose programming language built around **empirical self-optimization, structured concurrency, verified memory domains, and native machine compilation**.

Syntra unifies the ergonomics of modern object-oriented and functional languages with zero-cost systems control, backed by an autonomous compiler intelligence engine and an adaptive self-profiling runtime.

---

## 🌟 Key Highlights

* **🧠 Empirical Self-Optimization (SLS & SAR):** The compiler and runtime feature integrated machine learning (`syn learn`) and a Self-Adapting Runtime (`syn run --sar`) that dynamically tracks guard failures, adjusts execution worker thread pools, pages memory pressure, and deoptimizes code paths automatically.
* **🔬 Autonomous Experimentation (SECE):** The compiler includes a sandboxed experimentation engine (`syn experiment`) that autonomously generates optimization candidates, benchmarks them in isolated processes, and tunes compilation flags along an empirical Pareto frontier.
* **🛡️ Verified Memory Domains (MEM-001..004):** Syntra guarantees memory safety through explicit memory domains (`strict`, `permissive`, `arena`) and linear ownership checks verified at compile time with zero garbage collection pauses.
* **⚡ Native Machine Code & Cranelift Backend:** Compile directly to standalone, stripped native executables (`syn build`) targeting Apple Silicon ARM64 Mach-O and Linux x86_64 ELF via Cranelift.
* **🧵 Built-in Structured Concurrency:** First-class async tasks (`Task.spawn`), FIFO synchronization channels (`Channel.new`, `Channel.select`), and bounded thread pools (`ThreadPool.new`).
* **🌐 Web & Real-Time Ready:** Built-in HTTP reactor, high-throughput web server, and full RFC 6455 WebSocket client/server protocol with XOR masking and SHA-1 framing.
* **🧰 First-Class Developer Tooling:** Full TypeScript IntelliSense editor extension for VS Code and Cursor (`syntra-lang`), automatic code formatting (`syn fmt`), language server (`syn lsp`), documentation generator (`syn doc`), and package manager (`syn lock`, `Package.syn`).

---

## 🚀 Quick Start

### 1. Build & Install the `syn` Toolchain

Syntra requires Rust 1.75+ (Cargo).

```bash
# Clone the repository
git clone https://github.com/syntra-lang/syntra.git
cd syntra

# Build the toolchain binary
cargo build --release --bin syn

# (Optional) Link to your PATH
cp target/release/syn ~/.cargo/bin/syn
```

### 2. Verify Installation

```bash
syn --version
# Output: syntra 0.1.0 (syntra-lang 2026 edition)
```

### 3. Create and Run Your First Project

```bash
# Initialize a new package
syn init my_project
cd my_project

# Run the program
syn run src/main.syn

# Compile to a native standalone binary
syn build src/main.syn --output my_app
./my_app
```

---

## 💻 Language Tour at a Glance

### Hello World & Standard I/O

```syntra
// examples/hello.syn
namespace Main {
    func main(): void {
        System.println("Hello, Syntra 2026!");
    }
}
```

### Object-Oriented Programming & Interfaces

```syntra
interface Printable {
    func describe(): string;
}

class User implements Printable {
    public int id;
    public string username;
    private bool active;

    new(int id, string username) {
        this.id = id;
        this.username = username;
        this.active = true;
    }

    public func describe(): string {
        return "User #" + this.id + ": " + this.username;
    }
}

func main(): void {
    let user = new User(101, "alex");
    System.println(user.describe());
}
```

### Structured Concurrency (Tasks & Channels)

```syntra
func worker(Channel<string> ch, int id): void {
    ch.send("Worker " + id + " completed task.");
}

func main(): void {
    let ch = Channel.new(5);

    for i in 0..3 {
        Task.spawn(fn() {
            worker(ch, i);
        });
    }

    for i in 0..3 {
        let msg = ch.recv();
        System.println("Received: " + msg);
    }
}
```

### Real-Time WebSocket Server

```syntra
import Web;

func main(): void {
    let server = new WebSocketServer(8080);
    server.on_message(fn(client, msg) {
        System.println("Client payload: " + msg);
        client.send_text("Echo: " + msg);
    });
    server.listen();
}
```

---

## 🏛️ System Architecture

Syntra is designed as a modular pipeline of 11 distinct crates:

```
                    ┌────────────────────────┐
                    │      Source (.syn)     │
                    └───────────┬────────────┘
                                │
                        [syntra_syntax]       (Lexer, Parser, AST, Tokens)
                                │
                        [syntra_frontend]     (Type Checker, Symbols, Loader)
                                │
                          [syntra_air]        (AIR Intermediate Representation & Invariants)
                                │
                         [syntra_opt]         (Optimization Pipeline, DCE, Inlining)
                                │
                   ┌────────────┴────────────┐
                   ▼                         ▼
         [syntra_runtime]          [syntra_backend_cranelift]
      (Interpreter, Domains,           (Native Machine Code,
      SAR Governor, WebSockets)        Mach-O / ELF Binaries)
                   ▲                         ▲
                   └────────────┬────────────┘
                                │
              ┌─────────────────┴─────────────────┐
              │                                   │
      [syntra_analysis]                  [syntra_intelligence]
 (Whole-Program Call Graph,          (Multi-Agent Planner, SLS Models,
  Escape, Effect Analysis)           Dataset Governance, ECE Eval)
              │                                   │
              └─────────────────┬─────────────────┘
                                │
                        [syntra_sandbox]      (SECE Autonomous Experimentation)
                                │
                      [syntra_provenance]     (Reproducible Build Manifests)
                                │
                          [syntra_cli]        (Unified CLI: syn)
```

### Workspace Crates Directory

| Crate | Path | Description |
| :--- | :--- | :--- |
| `syntra_syntax` | [`crates/syntax/`](file:///Users/Topman/Documents/Projects/aether/crates/syntax/) | Lexer, AST nodes, tokens, and recursive-descent parser. |
| `syntra_diagnostics` | [`crates/diagnostics/`](file:///Users/Topman/Documents/Projects/aether/crates/diagnostics/) | Source code spans, formatted terminal errors, and diagnostic reporting. |
| `syntra_frontend` | [`crates/frontend/`](file:///Users/Topman/Documents/Projects/aether/crates/frontend/) | Type checker, symbol tables, module resolver, and manifest loader. |
| `syntra_air` | [`crates/air/`](file:///Users/Topman/Documents/Projects/aether/crates/air/) | Aether/Syntra Intermediate Representation, SSA lowering, and safety verifier. |
| `syntra_opt` | [`crates/opt/`](file:///Users/Topman/Documents/Projects/aether/crates/opt/) | Optimization pipeline, constant folding, dead-code elimination, and inlining. |
| `syntra_runtime` | [`crates/runtime/`](file:///Users/Topman/Documents/Projects/aether/crates/runtime/) | Bytecode evaluation engine, SAR adaptive governor, memory domains, and WebSockets. |
| `syntra_backend_cranelift`| [`crates/backend_cranelift/`](file:///Users/Topman/Documents/Projects/aether/crates/backend_cranelift/) | Native machine code compilation targeting ARM64 and x86_64. |
| `syntra_analysis` | [`crates/analysis/`](file:///Users/Topman/Documents/Projects/aether/crates/analysis/) | Whole-program call graph, escape analysis, and incremental invalidation. |
| `syntra_intelligence` | [`crates/intelligence/`](file:///Users/Topman/Documents/Projects/aether/crates/intelligence/) | SLS machine learning models, governance, multi-agent planner, and evaluation. |
| `syntra_sandbox` | [`crates/sandbox/`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/) | Autonomous candidate generation, sandbox process benchmarks, and Pareto scoring. |
| `syntra_provenance` | [`crates/provenance/`](file:///Users/Topman/Documents/Projects/aether/crates/provenance/) | Cryptographic digests, hermetic builds, and provenance manifests. |
| `syntra_cli` | [`crates/cli/`](file:///Users/Topman/Documents/Projects/aether/crates/cli/) | Main CLI driver binary (`syn`) integrating all subcommands. |

---

## 🛠️ Unified CLI Reference (`syn`)

```bash
syn <COMMAND> [OPTIONS] [FILE]
```

* `syn run <file>`: Execute program using the interpreter.
  * Add `--sar` to enable the Self-Adapting Runtime governor with dynamic worker scaling and memory pacing.
* `syn build <file>`: Compile to a standalone native binary via Cranelift.
* `syn check <file>`: Run static type checking and safety verification without execution.
* `syn test`: Run all unit and integration tests defined across the package.
* `syn fmt <file>`: Reformat code to canonical style guidelines.
* `syn lsp`: Start the standard JSON-RPC Language Server for editors.
* `syn doc <file>`: Generate Markdown API documentation for namespaces, classes, and methods.
* `syn plan <file>`: Query the Syntra Intelligence Engine (SIE) for optimization strategy recommendations.
* `syn experiment <file>`: Run the Autonomous Experimentation Engine (SECE) to benchmark sandbox candidates.
* `syn learn <train|eval|list|rollback>`: Train, evaluate, and manage empirical machine learning models.
* `syn lock`: Resolve dependencies and generate `Package.lock`.
* `syn init <name>`: Create a new Syntra package scaffold with `Package.syn`.

---

## 🧩 VS Code & Cursor Extension (`editors/vscode/syntra-lang/`)

Syntra features a production-grade, 100% TypeScript IntelliSense extension:

* **Syntax Highlighting & Grammar:** Comprehensive TextMate grammar covering keywords, types, lambdas, strings, and interpolation.
* **Context-Aware Autocomplete:** Automatic completions for standard libraries (`System`, `Math`, `File`, `Json`, `Task`, `Channel`, `ThreadPool`), member access methods, types, and keywords.
* **Package Manifest IntelliSense:** Dedicated auto-completion, parameter hints, and hover docs inside `Package.syn`.
* **Hover Documentation:** Instant Markdown API docs and code examples on hover.
* **Signature Help:** Real-time parameter hints when calling functions and methods.
* **Document Outline & Breadcrumbs:** Structured navigation for classes, interfaces, methods, and manifest blocks.
* **Integrated Commands Palette:** Quick commands for running, building, testing, and formatting.

To install the extension locally:
```bash
./install_extension.sh
```

---

## 📚 Comprehensive Documentation

Full documentation is available in the [`docs/`](docs/) directory:

* [**Getting Started Guide**](docs/getting-started/quickstart.md)
* [**Installation & Toolchain Setup**](docs/getting-started/installation.md)
* [**Language Reference Manual**](docs/language/index.md)
* [**Standard Library API Documentation**](docs/stdlib/index.md)
* [**Package Manager & Package.syn Specification**](docs/tools/package-manifest.md)
* [**Self-Adapting Runtime (SAR) Internals**](docs/advanced/sar-adaptive-runtime.md)
* [**Syntra Learning System (SLS) Architecture**](docs/advanced/sls-learning-system.md)
* [**Autonomous Experimentation (SECE) Guide**](docs/advanced/sece-autonomous-experimentation.md)
* [**Self-Hosting Compiler Guide**](docs/advanced/self-hosting.md)

---

## 🧪 Testing the Workspace

To run the complete test suite across all 11 crates:

```bash
cargo test --workspace
```

---

## 📄 License

Syntra is open-source software licensed under the MIT License.
