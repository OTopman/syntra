# Syntra Engineering Remediation: Phase 4 — Native Runtime & Concurrency Completeness

**Programme Title:** Syntra Comprehensive Engineering Remediation and Stabilisation Programme  
**Phase:** Phase 4 — Native Runtime & Concurrency Completeness (FIND-003, FIND-004)  
**Lead Engineer:** Principal Systems Architect & Runtime Systems Lead  
**Status:** **PHASE 4 COMPLETE (VERIFIED)**  
**Date:** 2026-11-24  

---

## 1. Executive Summary & Defect Scope

Phase 4 addressed two foundational deficiencies in the Syntra native compilation pipeline and runtime library (`crates/runtime/native/syntra_rt.c`):

1. **FIND-003 (CRITICAL):** Missing native runtime linker symbol `_syntra_channel_select`. While AIR lowering emitted `syntra_channel_select` for `Channel.select(channels)` and `syntra_backend_cranelift` declared the import symbol, `syntra_rt.c` never implemented the function, causing native link failures on any code utilizing channel multiplexing.
2. **FIND-004 (HIGH):** Permanent native heap memory leaks. `syntra_alloc` simply forwarded to `calloc(1, size)` without garbage collection, reference counting, object headers, or deallocation hooks, causing monotonic RSS memory growth and process exhaustion on long-running services.

### Root Cause Analysis

#### FIND-003:
In `crates/air/src/lowering.rs:3162`, static method invocations of `Channel.select(channels)` lowered to:
```rust
Op::Call {
    func_name: "syntra_channel_select".to_string(),
    args: vec![channels_val],
}
```
In `crates/backend_cranelift/src/compiler.rs:1010-1017`, `syntra_channel_select` was declared as an external import. However, in `crates/runtime/native/syntra_rt.c`, only `syntra_channel_new`, `syntra_channel_send`, `syntra_channel_recv`, `syntra_channel_close`, `syntra_channel_len`, `syntra_channel_try_recv`, and `syntra_channel_try_send` were present. When compiling `examples/advanced_concurrency_test.syn` with `syn build`, the linker failed with:
```
Undefined symbols for architecture x86_64:
  "_syntra_channel_select", referenced from: _main in adv_conc_bin.o
clang: error: linker command failed with exit code 1
```

#### FIND-004:
Specification 02 dictates safe-code memory management across four memory domains (Stack, Arena, Reference Counted, and Tracing Managed). The native C runtime only implemented basic arena slabs (`syntra_arena_alloc`), while all general object allocations, class instantiations, and string operations fell back to `syntra_alloc(size) = calloc(1, size)`. There was no memory accounting, no object headers, no garbage collector, and no `syntra_free` function, permanently leaking all heap objects.

---

## 2. Technical Remediation Implementation

### A. Thread-Safe Channel Multiplexing (`syntra_channel_select`)
In `crates/runtime/native/syntra_rt.c`, implemented `int64_t syntra_channel_select(void* array_ptr)`:
```c
int64_t syntra_channel_select(void* array_ptr) {
    if (!array_ptr) return -1;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (arr->length <= 0 || !arr->elements) return -1;

    while (1) {
        for (int64_t i = 0; i < arr->length; i++) {
            syntra_channel_t* ch = (syntra_channel_t*)arr->elements[i];
            if (!ch) continue;
            pthread_mutex_lock(&ch->lock);
            if (ch->size > 0 || ch->closed) {
                pthread_mutex_unlock(&ch->lock);
                return i;
            }
            pthread_mutex_unlock(&ch->lock);
        }
#ifdef _WIN32
        Sleep(1);
#else
        usleep(1000);
#endif
    }
    return -1;
}
```
- **Semantics:** Polling across the array of `syntra_channel_t*` pointers while acquiring per-channel mutex locks.
- **Immediate Return:** Returns the 0-based index of the first channel with pending messages (`ch->size > 0`) or closed state (`ch->closed`).
- **Cooperative Yield:** Sleeps 1ms between polling sweeps to prevent CPU spinning, identical to interpreter semantics in `crates/runtime/src/eval.rs:1298-1307`.
- **Defensive Guards:** Safely handles null array pointers, empty arrays, null channels, and closed channels.

### B. Managed Heap & Mark-and-Sweep Garbage Collection Subsystem
Implemented a complete, high-performance tracing garbage collector and tracked memory manager in `crates/runtime/native/syntra_rt.c`:

1. **Aligned GC Header:**
   ```c
   #define SYNTRA_GC_MAGIC 0x53594E54 // "SYNT"
   #define SYNTRA_GC_FLAG_MARKED 0x01
   #define SYNTRA_GC_FLAG_ARRAY  0x02

   typedef struct syntra_gc_header {
       struct syntra_gc_header* next;
       struct syntra_gc_header* prev;
       size_t size;
       uint32_t flags;
       uint32_t magic;
   } syntra_gc_header_t;
   ```
   - 32-byte header ensuring 16-byte alignment of all allocated object payloads `(hdr + 1)`.

2. **Thread-Safe Allocation & Automatic Collection Trigger:**
   - Synchronized via `pthread_mutex_t g_gc_lock`.
   - Tracks `g_gc_allocated_bytes`, `g_gc_peak_bytes`, `g_gc_total_allocations`, `g_gc_total_freed`, and `g_gc_collection_count`.
   - Automatically triggers `syntra_gc_collect_internal()` whenever `g_gc_allocated_bytes + alloc_sz > g_gc_threshold` (default 16MB, configurable).

3. **Dual Root Scanning Strategy:**
   - **Explicit Shadow Stack:** `syntra_gc_push_root(void**)` and `syntra_gc_pop_root()` for registered frame roots.
   - **Conservative Stack Frame Scanning:** Scans the active native thread stack between the current frame `__builtin_frame_address(0)` and the entry stack base `g_stack_base`.
   - Safe word alignment and bounds clamping (max 1MB scan depth) to avoid memory faults.

4. **Recursive Reference Marking & Dynamic Arrays:**
   - Recursively walks interior pointer fields of marked objects.
   - Specifically handles `syntra_array_t` objects (`SYNTRA_GC_FLAG_ARRAY`), scanning individual element pointers `arr->elements[i]`.

5. **Sweep & Reclamation:**
   - Unlinks unreferenced objects from the doubly linked global list `g_gc_head`.
   - Automatically frees dynamic array buffers (`arr->elements`).
   - Reclaims memory via `free(hdr)` and decrements `g_gc_allocated_bytes`.

6. **Deterministic Free & Introspection APIs:**
   - `syntra_free(void* ptr)`: Allows explicit RAII/destructor deallocation of tracked objects.
   - `syntra_memory_allocated_bytes()`: Returns currently live allocated heap bytes.
   - `syntra_gc_total_collections()`: Returns count of GC collection cycles completed.
   - `syntra_gc_collect()`: Explicit GC invocation.
   - `syntra_gc_set_threshold(int64_t)`: Sets automatic collection threshold.

---

## 3. Verification & Regression Evidence

### A. Advanced Concurrency Test (`examples/advanced_concurrency_test.syn`)
Compiled directly to a standalone native binary and executed:
```bash
cargo run --bin syn -- build examples/advanced_concurrency_test.syn -o /tmp/adv_conc_bin
/tmp/adv_conc_bin
```
**Output:**
```
🔨 Building native binary for 'examples/advanced_concurrency_test.syn' (target: x86_64-apple-darwin)...
🎯 PGO: Consuming profile lockfile 'profile.synlock' (Integrity: 1c82801ac39c219e)
⚡ Optimization pipeline applied 1 pass(es) with verified invariants.
🔒 Build Identity Manifest: 8cf58453f609afde
✅ Successfully compiled native executable: /tmp/adv_conc_bin
🚀 Run it directly: .//tmp/adv_conc_bin
--- Testing Advanced Concurrency ---
1. Initializing ThreadPool with 4 workers...
   Task 1 result: 25
   Task 2 result: 64
   Task 3 result: 70
   ThreadPool successfully shut down.
2. Testing Non-Blocking Channel operations...
   tryRecv on empty channel: 0
   trySend results: true, true
   tryRecv results: 100, 200
3. Testing Channel.select across multiple channels...
   Ready channel index: 1
   Received value from selected channel: 42
--- Advanced Concurrency Test PASSED! ---
```
Exit Code: `0`.

### B. New Regression Tests in `crates/backend_cranelift/src/lib.rs`
Added three regression tests:
1. `test_native_channel_select_concurrency`: Builds and runs native binary verifying `Channel.select` across multiple channels with async worker tasks.
2. `test_native_gc_reclamation_loop`: Compiles native binary instantiating 500 OOP class instances in a loop, verifying execution without segfaults or unbounded leaks.
3. `test_native_runtime_gc_direct_c_api`: Comprehensive C FFI test verifying initial memory (0 bytes), allocation tracking, GC sweep of unreferenced allocations, and explicit `syntra_free` deallocation.

**Cranelift Backend Test Suite:**
```
running 8 tests
test tests::test_native_cross_target_isa_arm64_and_x86_64 ... ok
test tests::test_native_runtime_gc_direct_c_api ... ok
test tests::test_native_compile_and_link_return_code ... ok
test tests::test_native_compile_void_main ... ok
test tests::test_native_gc_reclamation_loop ... ok
test tests::test_native_compile_float_arithmetic ... ok
test tests::test_native_compile_function_calls_and_recursion ... ok
test tests::test_native_channel_select_concurrency ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.75s
```

### C. Workspace Non-Regression Baseline
Full workspace test run (`cargo test --workspace`):
- **Passing Tests:** **74 / 74** (up from baseline 59 -> Phase 1: 63 -> Phase 2: 68 -> Phase 3: 71 -> Phase 4: 74).
- **Flaky / Ignored / Failed:** **0**.
- **Empirical Artifacts:**
  - `remediation_output/evidence/phase4_channel_select_compile.txt`
  - `remediation_output/evidence/phase4_backend_cranelift_tests.txt`
  - `remediation_output/evidence/phase4_workspace_tests.txt`

---

## 4. Phase Completion Gate

| Acceptance Criteria | Status | Evidence |
| :--- | :--- | :--- |
| `syntra_channel_select` symbol implemented and exported | **PASSED** | `crates/runtime/native/syntra_rt.c:1754-1779` |
| Native channel multiplexing links and executes | **PASSED** | `examples/advanced_concurrency_test.syn` exit code 0 |
| Native heap allocations tracked and bounded | **PASSED** | `syntra_alloc`, `syntra_gc_collect`, `syntra_free` in `syntra_rt.c` |
| Tracing garbage collection collects unreferenced objects | **PASSED** | `test_native_runtime_gc_direct_c_api` passes |
| Zero regressions across workspace test suite | **PASSED** | 74/74 tests passing |

**Phase 4 is officially CLOSED and VERIFIED.** Ready to proceed to Phase 5.
