# Syntra Remediation: Phase 8 — Self-Hosting Compiler Recovery

**Deliverable ID:** `remediation_output/08-self-hosting-compiler-remediation.md`  
**Phase:** Phase 8 (Self-Hosting Compiler Recovery)  
**Finding Addressed:** **FIND-008** (Broken loop variable reassignment codegen in self-hosting compiler)  
**Status:** **CLOSED — EMPIRICALLY VERIFIED**  
**Evidence Artifacts:**
- `remediation_output/evidence/phase8_control_flow_asm.txt`
- `remediation_output/evidence/phase8_control_flow_execution.txt`
- `remediation_output/evidence/phase8_self_hosting_test.txt`
- `remediation_output/evidence/phase8_workspace_tests.txt`

---

## 1. Executive Summary

Phase 8 addressed and resolved finding **FIND-008**: a critical defect in the Syntra self-hosting compiler toolchain (`stdlib/air.syn`, `stdlib/codegen.syn`, and `src/syn_self.syn`).

In the original audit, compiling loops with variable reassignment (e.g. `while x > 10 { x = x - 1; }` in `examples/control_flow_test.syn`) via the self-hosting compiler `syn_self` produced native executables that entered permanent, unrecoverable infinite loops.

Following our root cause analysis and targeted code generation fix:
- `stdlib/air.syn` now emits a dedicated `mov` SSA instruction preserving the variable's canonical stack slot upon reassignment.
- `stdlib/codegen.syn` now implements `mov` instruction lowering, generating the required x86-64 `mov qword ptr [rbp - slot], rax` write-back store instruction.
- The compiled `cf_via_self` binary terminates cleanly in under 10ms with the correct exit status `5`.
- An automated integration test `crates/cli/tests/test_self_hosting.rs` was added to permanently prevent loop variable codegen regressions.
- All 82 workspace tests pass.

---

## 2. Root Cause Analysis

### 2.1 Missing Variable Write-Back in Codegen
In `stdlib/air.syn` (line 770), assignment expressions were handled as:
```syntra
if (expr.kind == "Assignment") {
    if (expr.childCount() >= 2) {
        AstNode target = expr.getChild(0);
        AirValue val = this.lowerExpression(expr.getChild(1));

        if (target.kind == "Identifier") {
            this.bindVar(target.name, val);
            return val;
        }
    ...
```
`this.bindVar` merely updated an in-memory compiler mapping `this.varBindings` associating `target.name` with `val` (a newly allocated SSA value ID, e.g. `v14` in stack slot `[rbp - 136]`). No instruction was emitted into the basic block to write the updated value back to the variable's canonical stack slot (`v0` in stack slot `[rbp - 8]`).

### 2.2 Control Flow Loop Variable Stagnation
In the loop condition block (`.L_main_bb7`), the loop condition `x > 10` had already been lowered to read `x`'s initial slot:
```assembly
.L_main_bb7:
    mov rax, 10
    mov qword ptr [rbp - 112], rax
    mov rax, qword ptr [rbp - 8]      ; Always reads original value 15
    cmp rax, qword ptr [rbp - 112]
    setg al
    movzx rax, al
    mov qword ptr [rbp - 120], rax
    cmp rax, 0
    jne .L_main_bb8
    jmp .L_main_bb9
```
In the loop body block (`.L_main_bb8`), the subtraction was computed into a temporary slot `[rbp - 136]`, but was never written back to `[rbp - 8]`:
```assembly
.L_main_bb8:
    mov rax, 1
    mov qword ptr [rbp - 128], rax
    mov rax, qword ptr [rbp - 8]      ; Load 15
    sub rax, qword ptr [rbp - 128]    ; Subtract 1 -> 14
    mov qword ptr [rbp - 136], rax   ; Stored to temporary slot v14 only!
    jmp .L_main_bb7                  ; Jump back without writing to [rbp - 8]
```
Upon jumping back to `.L_main_bb7`, `[rbp - 8]` remained unconditionally `15`, causing `15 > 10` to evaluate to true indefinitely.

### 2.3 Absence of Move / Store Opcode in Native Code Generator
`stdlib/codegen.syn` contained zero handling for `mov` or `store` instructions in `lowerInstruction()`. Even if AIR had emitted a move instruction, the codegen would have silently ignored it.

---

## 3. Implementation Details

### 3.1 Emitting Move Instructions in `stdlib/air.syn`
In `stdlib/air.syn`, when an assignment target is an identifier, the lowerer looks up the variable's canonical storage value `orig`. If found, it emits a `mov` instruction copying `val` into `orig`:
```syntra
        if (expr.kind == "Assignment") {
            if (expr.childCount() >= 2) {
                AstNode target = expr.getChild(0);
                AirValue val = this.lowerExpression(expr.getChild(1));

                if (target.kind == "Identifier") {
                    AirValue orig = this.lookupVar(target.name);
                    if (orig.id >= 0) {
                        this.emit("mov", val.toString(), orig.toString(), target.name, val.ty, expr.span);
                        return orig;
                    } else {
                        this.bindVar(target.name, val);
                        return val;
                    }
                } else if (target.kind == "MemberAccess") {
                    AirValue obj = this.lowerExpression(target.getChild(0));
                    this.emit("setfield", obj.toString(), val.toString(), target.name, Type.Void(), expr.span);
                    return val;
                }
            }
        }
```

### 3.2 Lowering Move Instructions in `stdlib/codegen.syn`
In `stdlib/codegen.syn:362`, implemented native x86-64 lowering for `mov`:
```syntra
        if (inst.op == "const.int") {
            block.emit("mov", "rax", inst.operand1);
            block.emit("mov", resSlot, "rax");
        } else if (inst.op == "mov") {
            int srcId = this.parseValueId(inst.operand1);
            int dstId = this.parseValueId(inst.operand2);
            block.emit("mov", "rax", this.getSlot(srcId));
            block.emit("mov", this.getSlot(dstId), "rax");
            block.emit("mov", resSlot, "rax");
        } else if (inst.op == "const.bool") {
```
This guarantees that `mov v_src, v_dst` loads `v_src` into `rax` and stores it into `v_dst`'s stack slot `[rbp - dst_offset]`.

---

## 4. Verification Evidence

### 4.1 Emitted Assembly Inspection (`cf_via_self.s`)
Inspection of the newly emitted assembly for `.L_main_bb8`:
```assembly
.L_main_bb8:
    mov rax, 1
    mov qword ptr [rbp - 128], rax
    mov rax, qword ptr [rbp - 8]
    sub rax, qword ptr [rbp - 128]
    mov qword ptr [rbp - 136], rax
    mov rax, qword ptr [rbp - 136]
    mov qword ptr [rbp - 8], rax      ; <--- Write-back to variable x confirmed!
    mov qword ptr [rbp - 144], rax
    jmp .L_main_bb7
```
`mov qword ptr [rbp - 8], rax` updates `x` in place on each iteration.

### 4.2 End-to-End Execution
Running `/tmp/cf_via_self`:
```
Exit status: 5
```
Execution finishes in `~4ms` with the exact expected status code `5`.

### 4.3 Automated Regression Test (`crates/cli/tests/test_self_hosting.rs`)
Added test `test_self_hosting_loop_variable_codegen_and_execution`:
1. Compiles `src/syn_self.syn` with the bootstrap compiler `syn build`.
2. Uses the compiled `syn_self` to compile `examples/control_flow_test.syn`.
3. Asserts presence of `mov qword ptr [rbp - 8], rax` in generated `.s` file.
4. Executes the binary with a 5s watchdog timer and asserts exit code `5`.
All steps pass (`test test_self_hosting_loop_variable_codegen_and_execution ... ok`).

### 4.4 Workspace Test Suite (82 Tests)
Total passing tests across the workspace: **82 passed, 0 failed, 0 ignored**.
- `syntra_air`: 2/2
- `syntra_analysis`: 3/3
- `syntra_backend_cranelift`: 9/9
- `syntra_cli`: 7/7 (+1 self-hosting regression test: `test_self_hosting`)
- `syntra_frontend`: 13/13
- `syntra_intelligence`: 8/8
- `syntra_runtime`: 22/22
- `syntra_sandbox`: 9/9
- `syntra_syntax`: 9/9

---

## 5. Architectural Alignment & Conclusion

- **Self-Hosting Maturity:** Self-hosting compilation now correctly generates control flow and loop variable mutations for x86-64 targets.
- **Stage 2 Status:** Stage 2 remains an in-progress milestone as documented in Specification 10, but the foundational code generator now produces mathematically and semantically correct loop execution.
- **Disposition:** **FIND-008 is fully RESOLVED and EMPIRICALLY VERIFIED.**
