# Unified CLI Reference (`syn`)

The `syn` binary is the unified command-line interface for the Syntra compiler, runtime, package manager, intelligence engine, and developer tooling.

---

## 📋 Command Summary

```
Usage: syn <COMMAND> [OPTIONS] [FILE]

Commands:
  run          Execute a Syntra source file with the interpreter
  build        Compile to a standalone native machine binary (Mach-O / ELF)
  check        Run static type checking and safety invariant verification
  test         Run all unit and integration test assertions
  fmt          Format source code according to canonical styling
  doc          Generate Markdown API documentation
  lsp          Start the Language Server Protocol (JSON-RPC) daemon
  plan         Consult the Syntra Intelligence Engine (SIE) for optimization plans
  experiment   Run the Autonomous Experimentation Engine (SECE)
  learn        Train, evaluate, and manage machine learning optimization models
  lock         Resolve dependencies and freeze Package.lock
  package      Package and inspect archives for publication
  init         Create a new Syntra package scaffold with Package.syn
  clean        Clean build caches, logs, and artifacts
  repl         Start an interactive Syntra REPL shell
```

---

## 🏃 `syn run`
Executes a `.syn` file using the AST bytecode interpreter.

```bash
syn run src/main.syn [args...]
```

### Options:
* `--sar`: Enables the **Self-Adapting Runtime (SAR)** governor. Activates dynamic guard validation, automatic deoptimization, and adaptive worker scaling.
* `--profile`: Enables execution counter profiling and updates `profile.synlock`.
* `--verbose`: Emits detailed execution trace logs.

---

## 🏗️ `syn build`
Compiles a Syntra program into a stripped, standalone native machine binary using Cranelift.

```bash
syn build src/main.syn -o my_app
```

### Options:
* `-o, --output <path>`: Output executable file name (default: binary name or `a.out`).
* `--target <triple>`: Target architecture (e.g. `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`).
* `--opt-level <0|1|2|3>`: Optimization aggressiveness tier.
* `--emit-air`: Prints lowered AIR intermediate representation before compilation.

---

## 🔍 `syn check`
Performs lexical parsing, AST generation, static type checking, symbol resolution, and AIR invariant checks (`AIR-001..003` and `MEM-001..004`) without running the code.

```bash
syn check src/main.syn
```

---

## 🧪 `syn test`
Discovers and executes all test functions and assertions in the current package:

```bash
syn test
```

---

## 🪄 `syn fmt`
Reformats Syntra source files with standardized indentation, brace style, and spacing:

```bash
syn fmt src/main.syn
```

---

## 📚 `syn doc`
Extracts doc-comments (`///`), namespaces, classes, fields, and methods, generating structured Markdown reference documentation:

```bash
syn doc src/main.syn --output docs/api.md
```

---

## 🔌 `syn lsp`
Starts the standard Language Server Protocol (LSP) over stdio with JSON-RPC Content-Length framing. Used by VS Code, Cursor, Neovim, and other editors:

```bash
syn lsp
```

---

## 💡 `syn plan`
Consults the Syntra Intelligence Engine (SIE) multi-agent planner to analyze code structure, call graphs, and hot loops, outputting an optimization strategy report:

```bash
syn plan src/main.syn
```

---

## 🔬 `syn experiment`
Launches the Autonomous Experimentation Engine (SECE). Generates candidate variants, benchmarks them in isolated sandboxed child processes, and selects optimal parameters along a Pareto efficiency frontier:

```bash
syn experiment src/main.syn
```

---

## 🧠 `syn learn`
Manages the Syntra Learning System (SLS) empirical machine learning models:

* `syn learn train <file>`: Trains ensemble models on historical workload execution telemetry.
* `syn learn eval <version>`: Evaluates a model against the promotion gate (RMSE, MAE, Spearman $\rho$, ECE calibration).
* `syn learn list`: Lists all registered model versions and their promotion status.
* `syn learn rollback <version>`: Reverts production model tier to a prior validated checkpoint.
* `syn learn revoke <version>`: Revokes a degraded model and quarantines its weights.
* `syn learn audit`: Prints the immutable governance audit log (`.syntra/governance_audit.log`).

---

## 🔒 `syn lock`
Resolves external dependencies declared in `Package.syn` and writes the deterministic `Package.lock` file:

```bash
syn lock
syn lock --check  # Verifies lockfile is up to date without modifying disk
```
