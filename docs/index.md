# Syntra Documentation Book

Welcome to the official documentation for the **Syntra Programming Language** (Edition 2026).

Syntra is a modern, high-performance programming language designed from the ground up for empirical self-optimization, zero-leak memory safety, structured concurrency, and native machine compilation.

---

## 🗺️ Documentation Map

### [1. Getting Started](getting-started/quickstart.md)
* [**Installation Guide**](getting-started/installation.md): Building the compiler from source, setting up the PATH, and platform support.
* [**Quickstart Tutorial**](getting-started/quickstart.md): Creating your first project, running programs, and compiling standalone native binaries.
* [**Editor Setup**](getting-started/editor-setup.md): Installing and configuring the VS Code & Cursor extension with TypeScript IntelliSense.

### [2. Language Reference Manual](language/index.md)
* [**Lexical Structure**](language/lexical-structure.md): Tokens, keywords, comments, string interpolation, and operators.
* [**Type System**](language/type-system.md): Primitives (`int`, `float`, `bool`, `string`, `void`), generics, inference with `let`, and `const`.
* [**Control Flow**](language/control-flow.md): Conditionals, iteration loops, pattern matching, exception handling (`try`/`catch`/`finally`).
* [**Functions & Closures**](language/functions-and-closures.md): Named functions (`func`), lambdas (`fn`), closures, parameter passing.
* [**Object-Oriented Programming**](language/object-oriented.md): Classes, constructors (`new`), visibility (`public`/`private`), interfaces, and polymorphism.
* [**Structured Concurrency**](language/async-and-concurrency.md): Asynchronous tasks (`Task.spawn`), FIFO channels (`Channel.new`, `Channel.select`), and thread pools.
* [**Memory Domains & Safety**](language/memory-domains.md): Strict, permissive, and arena memory domains, linear ownership, and zero-leak invariants.

### [3. Standard Library Reference](stdlib/index.md)
* [**System & I/O**](stdlib/system-and-io.md): Standard output, process lifecycle, command-line arguments, environment variables.
* [**Math & File Operations**](stdlib/math-and-file.md): IEEE-754 arithmetic, trigonometry, file reading and writing.
* [**Collections & JSON**](stdlib/collections-and-json.md): Dynamic arrays, hash maps, high-speed JSON serialization and deserialization.
* [**Networking & WebSockets**](stdlib/networking-and-websockets.md): Built-in HTTP reactor, HTTP server, RFC 6455 WebSockets, Base64, and SHA-1.
* [**Concurrency Primitives**](stdlib/concurrency-primitives.md): In-depth guide to `Task`, `Channel`, and `ThreadPool`.

### [4. Toolchain & Package Management](tools/cli-reference.md)
* [**CLI Reference**](tools/cli-reference.md): Detailed reference for all `syn` subcommands (`run`, `build`, `check`, `test`, `fmt`, `doc`, `lsp`, `plan`, `experiment`, `learn`, `lock`).
* [**Package Manifest Specification**](tools/package-manifest.md): Complete guide to `Package.syn`, `Package.lock`, dependency formats (version, path, Git), and build flags.

### [5. Advanced Subsystems & Architecture](advanced/sar-adaptive-runtime.md)
* [**Self-Adapting Runtime (SAR)**](advanced/sar-adaptive-runtime.md): Dynamic guard validation, deoptimization, frame reconstruction, code cache LRU, and worker scaling.
* [**Syntra Learning System (SLS)**](advanced/sls-learning-system.md): Supervised ensemble models, ECE calibration, dataset governance, model promotion gate, and rollback.
* [**Autonomous Experimentation (SECE)**](advanced/sece-autonomous-experimentation.md): Sandboxed candidate generation, isolated process benchmarks, and Pareto frontier tuning.
* [**AIR Intermediate Representation**](advanced/air-intermediate-representation.md): AST lowering, SSA-like intermediate representation, and safety invariants (AIR-001..003).
* [**Native Cranelift Backend**](advanced/cranelift-native-backend.md): Low-level machine code emission, Mach-O and ELF generation, ABI conventions.
* [**Whole-Program Analysis**](advanced/whole-program-analysis.md): Interprocedural call graphs, escape analysis, effect analysis, and incremental invalidation.
* [**Self-Hosting Compiler**](advanced/self-hosting.md): Bootstrap architecture of the pure-Syntra self-hosting compiler (`src/syn_self.syn`).
