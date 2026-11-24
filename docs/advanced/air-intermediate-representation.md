# Aether/Syntra Intermediate Representation (AIR)

The Aether/Syntra Intermediate Representation (AIR) is an explicit, SSA-like intermediate representation defined in `crates/air/`. AIR sits between the AST type-checker and the native/interpreter backends, providing a canonical form for verification and optimization.

---

## 🎯 AIR Design Principles

* **Explicit Control Flow:** Structured control flow (`if`, `while`, `for`) is lowered into explicit basic blocks with conditional branch (`BrIf`) and unconditional jump (`Jmp`) instructions.
* **SSA-Style Register Operands:** Computations use virtual registers (`%v0`, `%v1`, `%v2`) with explicit assignments.
* **Invariant Enforcing:** AIR enforces strict semantic and safety rules before any code generation or optimization can proceed.

---

## 🛡️ AIR Safety Invariants

The AIR verifier (`syntra_air::verifier`) guarantees three fundamental invariants:

### 1. AIR-001: Register Single Definition & Dominance
* Every virtual register must be defined before use.
* In basic block control flow graphs, definitions must strictly dominate all uses.

### 2. AIR-002: Memory Domain Boundary Invariance
* Cross-domain references must be explicitly labeled as moves, borrows, or clones.
* An instruction in domain $D_1$ cannot dereference an interior pointer residing in domain $D_2$ without an explicit domain crossing bridge.

### 3. AIR-003: Basic Block Termination Invariance
* Every basic block must end with exactly one terminator instruction (`Ret`, `Jmp`, `BrIf`, or `Throw`).
* Instructions following a terminator are flagged as unreachable dead code and rejected.

---

## 📋 Lowered AIR Example

Consider this Syntra source code:
```syntra
func compute(int a, int b): int {
    let result = a * 2 + b;
    return result;
}
```

Lowered into AIR text format:
```air
module Main {
    func compute(%arg0: int, %arg1: int): int {
      bb0:
        %v0 = const.int 2
        %v1 = mul %arg0, %v0
        %v2 = add %v1, %arg1
        ret %v2
    }
}
```

You can inspect the lowered AIR representation of any program using:
```bash
syn check examples/hello.syn --emit-air
```
