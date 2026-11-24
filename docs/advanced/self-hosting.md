# Self-Hosting Compiler

A key milestone of the Syntra project is its pure-Syntra self-hosting compiler, located in [`src/syn_self.syn`](file:///Users/Topman/Documents/Projects/aether/src/syn_self.syn).

---

## 🐣 What is Self-Hosting?

A compiler is self-hosting when it can compile its own source code. Syntra achieves this through a two-stage bootstrap architecture:

```
[Stage 0: Rust Compiler (syn)]
             │
             ▼ Compiles
[src/syn_self.syn (Written in pure Syntra)]
             │
             ▼ Produces
[Stage 1: Standalone Syntra Binary (syn_self)]
             │
             ▼ Compiles
[Any Syntra Program or Next Compiler Version]
```

---

## 🧩 Structure of `src/syn_self.syn`

The self-hosting compiler is written entirely in Syntra, demonstrating the language's capabilities for large-scale systems engineering:

1. **Self-Hosting Lexer (`stdlib/syntax.syn`):** Tokenizes Syntra source code, handling numbers, identifiers, strings, and operators.
2. **Self-Hosting AST Nodes:** Defines `Program`, `Item`, `Function`, `Class`, `Stmt`, and `Expr` classes.
3. **Self-Hosting Parser:** Recursive-descent parser building the AST from token streams.
4. **Self-Hosting Type Checker (`stdlib/checker.syn`):** Resolves symbols, checks type compatibility, and verifies method signatures.
5. **Self-Hosting AIR Lowering (`stdlib/air.syn`):** Converts AST nodes into basic blocks, SSA registers, and branch instructions.
6. **Self-Hosting Code Generator (`stdlib/codegen.syn`):** Generates target machine code and executable binaries.

---

## 🔄 Verifying the Bootstrap

To run the self-hosting compiler using the Stage 0 Rust driver:

```bash
syn run src/syn_self.syn examples/hello.syn
```

**Output:**
```
🐣 Syntra Self-Hosting Compiler (syn_self v0.1.0)
[1/4] Lexing examples/hello.syn ... OK
[2/4] Parsing AST nodes ........... OK
[3/4] Type checking ............... OK
[4/4] Lowering to AIR ............. OK
✅ Compilation completed successfully!
```
