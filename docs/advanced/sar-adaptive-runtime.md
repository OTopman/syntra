# Self-Adapting Runtime (SAR)

The Syntra Adaptive Runtime (SAR) is an autonomous execution governor that dynamically adapts compiled code, speculative guards, concurrency thread pools, and memory allocation strategies to live production workloads.

---

## 🏛️ SAR Architecture

SAR operates as a feedback loop between the execution interpreter/JIT and background compilation workers:

```
                  ┌────────────────────────────────────────┐
                  │          Running Workload              │
                  └───────────────────┬────────────────────┘
                                      │
                         [Dynamic Guard Verification]
                         (Type, Range, NonNull, LoopBound)
                                      │
                   ┌──────────────────┴──────────────────┐
                   ▼                                     ▼
            [Guard Passed]                       [Guard Violated]
         Continue Fast Path              Trigger Deoptimization (RUN-021)
                                                         │
                                               [Deopt Manager]
                                         Reconstruct Execution Frame
                                         Record Violation Frequency
                                                         │
                                        ┌────────────────┴────────────────┐
                                        ▼                                 ▼
                              [Violations > Threshold]         [Memory Pacing Governor]
                             Blacklist Optimistic Spec       Pace Allocation Frequency
                             Queue Recompilation Request      Scale Worker Threads (RUN-022)
```

---

## 🛡️ Dynamic Guards (`crates/runtime/src/guards.rs`)

SAR uses speculative optimization guarded by low-overhead runtime invariant checks:

1. **`TypeGuard`:** Verifies that a generic or dynamic reference matches the monomorphic type assumed by the optimized path.
2. **`RangeGuard`:** Verifies that an integer or index remains within predicted bounds (e.g. `0 <= index < length`), eliminating array bounds checks.
3. **`NonNullGuard`:** Asserts that an object reference is non-null.
4. **`MonomorphicReceiverGuard`:** Asserts that a polymorphic method call site dispatches to the single expected receiver class.
5. **`LoopBoundGuard`:** Verifies that loop trip counts do not exceed vectorization or unrolling thresholds.

If a guard fails, the runtime initiates deoptimization.

---

## ↩️ Deoptimization & State Reconstruction (`crates/runtime/src/deopt.rs`)

When an optimistic assumption is violated:
* The runtime deconstructs the native frame into a portable `ContinuationFrame`.
* Local variable registers, operand stack items, and instruction pointers are reconstructed without process aborts or memory leaks.
* Execution resumes seamlessly in the baseline interpreter.
* **Anti-Thrashing Blacklist (RUN-021):** If a method fails guards repeatedly ($N \ge 3$), the optimizer permanently blacklists the speculative optimization for that callsite to prevent deopt loops.

---

## 💾 Code Cache & Active Frame Pinning (`crates/runtime/src/code_cache.rs`)

Compiled native code fragments are cached in the `CodeCache` under LRU eviction:
* **Active Frame Pinning (RUN-025):** Code blocks currently executing on thread stacks are pinned. Even under memory pressure, the cache will never evict an active frame.
* **LRU Eviction:** Cold, unpinned compiled methods are evicted when the code cache reaches its memory limit.

---

## 🎛️ Memory Pacing Governor & Worker Scaling (`crates/runtime/src/sar.rs`)

* **Default-Off Safety (RUN-020):** SAR is disabled by default and enabled explicitly via `--sar` in the CLI or `"syntra.sar.enabled": true` in configuration.
* **Worker Scaling (RUN-022):** Dynamically scales background worker thread pools between $1$ and $16$ workers based on queue latency and CPU core saturation.
* **Fault Injection:** Includes deterministic fault injection hooks for testing deopt resilience under simulated memory pressure and network latency spikes.
