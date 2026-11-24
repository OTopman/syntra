# Whole-Program Analysis

The Whole-Program Analysis subsystem (`crates/analysis/`) performs interprocedural static analysis across all files and modules in a Syntra package.

---

## 🔍 Interprocedural Call Graph

The call graph builder traverses the entire program AST to identify:
* Direct and indirect function call chains.
* Recursive functions and call cycle SCCs (Strongly Connected Components).
* Root entry points (`main`) and dead or unreferenced functions.

```rust
let callgraph = CallGraphBuilder::build(&program);
let callers = callgraph.get_callers("Worker.process");
```

---

## 🏃 Escape Analysis

Escape analysis determines whether an allocated object can outlive the stack frame of its creating function:
* **No Escape:** If an object never escapes its declaring scope, the compiler allocates it on the thread stack or reclaims it deterministically without heap allocation.
* **Domain Escape:** If an object is returned or passed into another domain, the analysis enforces domain ownership transfer or cloning.

---

## ⚡ Effect Analysis & I/O Detection

The effect analyzer classifies functions according to their side effects:
* **Pure Functions:** Functions that perform only deterministic mathematical computations without modifying external state or performing I/O. Safe for memoization, speculative execution, and aggressive constant folding.
* **I/O Functions:** Functions that interact with the filesystem (`File.*`), network (`Web.*`), or system clock (`System.*`). The optimizer guarantees that I/O operations are never reordered or eliminated across sequence points.

---

## 🔄 Incremental Invalidation Cache

To ensure fast compilation during active development:
* The analysis engine computes cryptographic SHA-256 digests of each source module.
* When a file changes, the **Incremental Invalidation Cache** marks only affected dependent functions for re-analysis, avoiding redundant whole-program sweeps.
