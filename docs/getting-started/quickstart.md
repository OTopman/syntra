# Quickstart Tutorial

This tutorial walks you through creating, running, type-checking, optimizing, and natively compiling your first Syntra program.

---

## 1. Initializing a New Package

Use the `syn init` command to create a standard Syntra package scaffold:

```bash
syn init my_first_app
cd my_first_app
```

This generates the following project layout:

```
my_first_app/
├── Package.syn      # Package manifest with configuration & dependencies
└── src/
    └── main.syn     # Application entrypoint
```

Let's examine [`Package.syn`](file:///Users/Topman/Documents/Projects/aether/Package.syn):

```syntra
// Package.syn - Syntra Package Manifest
package Config {
    name = "my_first_app"
    version = "0.1.0"
    edition = "2026"

    optimization {
        pgo = "locked"
        telemetry_mode = "sampling"
    }

    build {
        mode = "native"
        memory_domains = "strict"
    }

    network {
        tls_provider = "pure"
    }

    dependencies = [
    ]
}
```

---

## 2. Writing Your First Code

Open `src/main.syn` and write:

```syntra
namespace Main {
    func fibonacci(int n): int {
        if n <= 1 {
            return n;
        }
        return fibonacci(n - 1) + fibonacci(n - 2);
    }

    func main(): void {
        System.println("=== Syntra Quickstart ===");
        let n = 10;
        let result = fibonacci(n);
        System.println("Fibonacci(" + n + ") = " + result);
    }
}
```

---

## 3. Running with the Interpreter

Execute your source file directly using the interpreter:

```bash
syn run src/main.syn
```

**Output:**
```
=== Syntra Quickstart ===
Fibonacci(10) = 55
```

---

## 4. Running with the Self-Adapting Runtime (SAR)

To enable the dynamic machine learning governor, dynamic guard tracking, and worker thread pool scaling, pass the `--sar` flag:

```bash
syn run src/main.syn --sar
```

The runtime will monitor memory pressure, dynamically deoptimize guarded speculative code paths if invariants are violated, and scale workers automatically.

---

## 5. Type-Checking & Verification

To verify that your code adheres to all static type rules and ownership constraints without executing it:

```bash
syn check src/main.syn
```

**Output:**
```
✅ Program verified successfully with 0 errors.
```

---

## 6. Compiling to a Native Machine Binary

Compile your Syntra program to a stripped, standalone native machine executable via the Cranelift backend:

```bash
syn build src/main.syn --output my_app
```

This creates the standalone executable `./my_app`. Execute it directly:

```bash
./my_app
```

No runtime, interpreter, or Rust toolchain is required to run `./my_app`.

---

## 7. Formatting Your Code

Format your code according to canonical Syntra formatting conventions:

```bash
syn fmt src/main.syn
```

Next, proceed to [**Editor Setup**](editor-setup.md) to configure VS Code or Cursor with full TypeScript IntelliSense.
