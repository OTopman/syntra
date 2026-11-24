# Autonomous Experimentation Engine (SECE)

The Syntra Experimental Compilation Engine (SECE) is an autonomous, empirical optimization candidate generator and benchmarking harness implemented in `crates/sandbox/`.

---

## 🔬 How SECE Works

Traditional compilers apply static heuristics to decide whether to unroll loops, inline functions, or vectorize code. SECE replaces static guesswork with **empirical experimentation**:

```
                       ┌─────────────────────────┐
                       │   Syntra Source (.syn)  │
                       └────────────┬────────────┘
                                    │
                       [Candidate Generation Engine]
                      (Varying inlining, loop unrolling,
                       vectorization, memory domains)
                                    │
                                    ├─── Candidate 1 (aggressive inlining)
                                    ├─── Candidate 2 (vectorized loops)
                                    └─── Candidate 3 (arena allocation)
                                    │
                         [Process Sandbox Runner]
                        (Isolated child process,
                         timeout watchdog, CPU affinity)
                                    │
                     [Multi-Objective Metrics Collector]
                    (Runtime, Memory, Code Size, Energy)
                                    │
                       [Pareto Frontier Optimizer]
                    (Selects non-dominated solutions)
                                    │
                       [Optimal Variant Deployment]
```

---

## 🛡️ Sandbox Process Isolation

To ensure that experimental variants never crash the host compiler or leak resources:
* **Child Process Sandboxing:** Each candidate is compiled and run in a separate child process.
* **Strict Timeouts:** Candidates exceeding execution thresholds (default 5000 ms) are terminated via the watchdog.
* **Resource Quotas:** Memory usage is bounded via OS resource limits (`RLIMIT_AS` on POSIX systems).
* **Fault Handling:** Segmentation faults, illegal instructions, or out-of-memory errors are trapped gracefully and recorded as `FailureReason::Crash` or `FailureReason::Timeout`.

---

## 📊 Pareto Frontier Optimization

SECE evaluates candidates across multiple competing objectives:
1. **Execution Time (Speedup):** Measured via high-resolution monotonic clocks.
2. **Peak Memory Consumption:** Measured via max resident set size (RSS).
3. **Binary Code Size:** Measured in bytes of compiled machine code.

A candidate $A$ dominates candidate $B$ if $A$ is better than or equal to $B$ in all metrics and strictly better in at least one. SECE extracts the **Pareto frontier** of non-dominated candidates and recommends the optimal candidate based on target deployment profiles (`performance`, `embedded`, or `balanced`).

---

## 💻 Running SECE via CLI

To run autonomous experimentation on any source file:

```bash
syn experiment examples/hello.syn
```

**Sample Output:**
```
🔬 Running Syntra Experimental Compilation Engine (SECE)...
Generated 4 candidate variants.
[1/4] Candidate 'baseline' ................. OK (speedup: 1.00x, mem: 4.2 MB)
[2/4] Candidate 'inline_aggressive' ........ OK (speedup: 1.18x, mem: 4.4 MB)
[3/4] Candidate 'arena_domains' ............ OK (speedup: 1.34x, mem: 3.8 MB)
[4/4] Candidate 'unroll_max' ............... OK (speedup: 1.12x, mem: 4.9 MB)

🏆 Selected Pareto-optimal candidate: 'arena_domains' (+34% speedup, -10% memory)
```
