# Syntra Engineering Remediation: Phase 3 — Language Semantics & Async/Await Correctness

**Programme Title:** Syntra Comprehensive Engineering Remediation and Stabilisation Programme  
**Phase:** Phase 3 — Language Semantics & Async/Await Correctness (FIND-005, FIND-006)  
**Lead Engineer:** Principal Compiler & Language Semantics Architect  
**Status:** **PHASE 3 COMPLETE (VERIFIED)**  
**Date:** 2026-11-24  

---

## 1. Executive Summary & Defect Scope

In Phase 3, the compiler engineering team resolved two related critical defects in Syntra's concurrency semantics, syntax parsing, and static type checking:

1. **FIND-005 (HIGH):** Async/await parser operator precedence flaw in `crates/syntax/src/parser.rs` causing corrupted pointer arithmetic in AIR lowering (`crates/air/src/lowering.rs`) and runtime deadlocks in `pthread_mutex_lock`.
2. **FIND-006 (MEDIUM):** Type checker wildcard bypass in `crates/frontend/src/checker.rs`, where `Expr::Await` unconditionally returned `Type::Unknown`, disabling static type checking for awaited asynchronous results and allowing non-Task types to be awaited silently.

### Root Cause Analysis

#### FIND-005:
In `crates/syntax/src/parser.rs`, the Pratt parser for expressions parsed prefix `await` using binding power 13:
```rust
// Legacy flawed parser logic in crates/syntax/src/parser.rs:888
Some(Token::Await) => {
    let await_span = self.peek_span();
    self.advance();
    let expr = self.parse_expression(13)?; // Flaw: 13 is lower than BinaryOp::Add (17, 18)
...
```
Because infix addition (`+`) has binding power `(17, 18) > 13`, parsing `await a + await b` caused `parse_expression(13)` to consume `+` and the entire subsequent expression as the operand to the first `await`.
- **Resulting AST:** `await (a + await b)`
- **Behavior in Interpreter:** Printed string concatenation `<Task #4>40` instead of integer sum `60`.
- **Behavior in Native Compiler:** Evaluated `await b` (= 40), then performed integer addition on Task heap pointer `a` + 40 bytes, passing an invalid offset memory address to `syntra_task_join`. This caused native execution to crash or deadlock in `pthread_mutex_lock`.

#### FIND-006:
In `crates/frontend/src/checker.rs:2613`:
```rust
// Legacy type checker bypass
Expr::Await { expr, .. } => {
    let _inner_ty = self.check_expression(expr);
    Type::Unknown
}
```
`Expr::Await` silenced all type errors, preventing the type checker from verifying that the awaited target was actually a `Task<T>`, and returning `Type::Unknown` so subsequent assignments and arithmetic were unverified.

---

## 2. Technical Remediation Implementation

### A. Parser Operator Precedence Correction
In `crates/syntax/src/parser.rs`:
- Elevated the prefix binding power of `await` from 13 to **21**.
- Since binary arithmetic operators have maximum binding power 20 (`*`, `/`, `%`), prefix `await` now binds tighter than all binary arithmetic, comparison, bitwise, and logical operators.
- `await a + await b` now correctly parses as:
  ```rust
  Expr::Binary {
      op: BinaryOp::Add,
      left: Box::new(Expr::Await { expr: a }),
      right: Box::new(Expr::Await { expr: b }),
  }
  ```

### B. Strongly Typed Task Inference in Type Checker
In `crates/frontend/src/checker.rs`:
1. **Async Function Signatures:** When declaring an async function (`func.is_async`), the checker registers its return type in `self.symbols.functions` as `Type::GenericInstance("Task", vec![inner_return_type])`, while verifying that `return` statements in the function body match `inner_return_type`.
2. **Await Type Unwrapping:**
   ```rust
   Expr::Await { expr, .. } => {
       let inner_ty = self.check_expression(expr);
       match inner_ty {
           Type::GenericInstance(ref name, ref args) if name == "Task" && !args.is_empty() => {
               args[0].clone() // Unwraps T from Task<T>
           }
           Type::Class(ref name) if name == "Task" => {
               Type::Unknown // Preserves backward compatibility for untyped Task
           }
           Type::Unknown => Type::Unknown,
           other => {
               self.error(
                   format!("'await' expression expects a Task, but found '{}'", other),
                   expr.span(),
               );
               Type::Unknown
           }
       }
   }
   ```
3. **Task Method Typing:** Calling `.await()`, `.join()`, or `.wait()` on `Task<T>` now properly unwraps and returns `T`.
4. **Invalid Await Rejection:** Non-Task expressions (e.g. `await 42`) are strictly rejected with an explicit compile-time type diagnostic.

### C. AIR Lowering Precision
In `crates/air/src/lowering.rs`:
- Updated `Expr::Await` lowering to inspect the operand's type. When lowering `Task<T>`, the instruction result type is mapped to `T` (rather than defaulting strictly to `Type::Int`), ensuring complete type preservation across the SSA IR.

---

## 3. Empirical Verification Evidence

### A. Syntax & Frontend Regression Unit Tests
1. **Parser Precedence Test:** Added `test_parse_await_precedence` in [`crates/syntax/src/lib.rs`](file:///Users/Topman/Documents/Projects/aether/crates/syntax/src/lib.rs), asserting that `await a + await b` parses into an outer `BinaryOp::Add` with two `Await` operands.
2. **Type Checking Tests:** Added in [`crates/frontend/src/lib.rs`](file:///Users/Topman/Documents/Projects/aether/crates/frontend/src/lib.rs):
   - `test_async_await_type_check_valid`: Verifies that `int r = await compute_int();` and `int sum = await a + await b;` type-check cleanly.
   - `test_async_await_type_mismatch_fails`: Verifies that assigning `await compute_int()` to `string` fails type checking, and awaiting a non-Task integer `await 42` produces a diagnostic error.

**Focused Crate Tests:**
```
running 9 tests in syntra_syntax ... ok (9 passed)
running 12 tests in syntra_frontend ... ok (12 passed)
```
*Evidence log: [`remediation_output/evidence/phase3_async_await_tests.txt`](file:///Users/Topman/Documents/Projects/aether/remediation_output/evidence/phase3_async_await_tests.txt)*

### B. Full Workspace Non-Regression Verification
Ran `cargo test --workspace`:
- Baseline passing tests: **59**
- Phase 1 passing tests: **63**
- Phase 2 passing tests: **68**
- Current passing tests: **71** (59 baseline + 4 in `cli` + 5 in `sandbox` + 1 in `syntax` + 2 in `frontend`)
- Failed tests: **0**
- Regressions: **0**

*Evidence log: [`remediation_output/evidence/phase3_workspace_tests.txt`](file:///Users/Topman/Documents/Projects/aether/remediation_output/evidence/phase3_workspace_tests.txt)*

### C. End-to-End Fixture Verification (`examples/async_await_test.syn`)
Both the tree-walking interpreter and Cranelift native executable were executed on the canonical test fixture:

1. **Interpreter (`syn run`):**
   ```
   --- Starting Async/Await Test ---
   Direct await compute_double(21) = 42
   Direct await greet_async = Hello, Syntra from async!
   Task handle await = 100
   Concurrent async sum (10*2 + 20*2) = 60
   Method syntax t2.await() = 200
   Async class method multiply(6, 7) = 42
   --- Async/Await Test Completed Successfully ---
   ```
2. **Native Cranelift Compiler (`syn build`):**
   Compiled to native executable `/tmp/async_await_test_bin` and executed:
   ```
   --- Starting Async/Await Test ---
   Direct await compute_double(21) = 42
   Direct await greet_async = Hello, Syntra from async!
   Task handle await = 100
   Concurrent async sum (10*2 + 20*2) = 60
   Method syntax t2.await() = 200
   Async class method multiply(6, 7) = 42
   --- Async/Await Test Completed Successfully ---
   ```
*Evidence log: [`remediation_output/evidence/phase3_async_await_native_run.txt`](file:///Users/Topman/Documents/Projects/aether/remediation_output/evidence/phase3_async_await_native_run.txt)*

---

## 4. Phase 3 Sign-Off & Transition Gate

| Criterion | Target | Actual | Status |
| :--- | :--- | :--- | :--- |
| `await` Operator Precedence | Higher than binary `+` | Precedence 21 (higher than `*`, `+`) | **MET** |
| Concurrent Async Sum | Integer `60` | Evaluated to `60` (both runtimes) | **MET** |
| Native Deadlock in `pthread_mutex_lock` | Zero deadlock | Clean termination in milliseconds | **MET** |
| Type Checker Task Type Unwrapping | Unpack `T` from `Task<T>` | Returns `T` on await | **MET** |
| Non-Task Await Type Safety | Diagnostic rejection | Rejects `await 42` with compile error | **MET** |
| Workspace Non-Regression | 68/68 passing | 71/71 passing | **MET** |

**Conclusion:** Phase 3 is **COMPLETE and VERIFIED**. FIND-005 and FIND-006 are closed. The platform is ready to transition to **Phase 4: Native Runtime & Concurrency Completeness (FIND-003, FIND-004)**.
