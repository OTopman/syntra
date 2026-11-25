# Syntra Phase 2: SECE Sandbox Security Architecture
# Deliverable 03B: Security Contract Addendum & Kernel Refinements

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox  
**Target Milestone:** Phase 2 Architectural Hardening (Review of Deliverable 03A)  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **AUTHORITATIVE SPECIFICATION ADDENDUM — READY FOR REVIEW**

---

## 1. Executive Summary & Purpose

This addendum resolves the five kernel-level and boundary-handling issues raised in the technical review of Deliverable 03A. Together with Deliverables 01, 02, and 03A, this document establishes the **authoritative, definitive architecture specification** for Phase 2 prior to adversarial test implementation.

### Key Refinements in Deliverable 03B:
1. **Linux Network Isolation:** Runtime verification of empty routing tables, loopback interface down/unconfigured state, and absence of veth/macvlan host interfaces.
2. **PID Namespace Supervision:** Dedicated supervisor thread binding for `PR_SET_PDEATHSIG`, combined with Linux 5.3+ `pidfd` tracking (`pidfd_open`, `pidfd_send_signal`, `poll` on pidfd) to eliminate thread-death races.
3. **Memory Accounting Semantics:** Renaming `memory_rss_ceiling` to `memory_allocation_ceiling` with precise accounting semantics: Cgroups v2 charged memory (anonymous + page cache + kernel structures) on Linux, and committed private virtual memory on Windows.
4. **Explicit Trust Boundaries for Guarded Mode:** Formal policy that `Guarded` mode is restricted to **developer-authored, trusted local workloads**. Execution of unverified third-party candidates or untrusted dependencies **strictly requires `Strict` mode and fails closed on native macOS and Windows**.
5. **Hardened Stream Supervisor:** Independent stdout and stderr byte budgets, explicit overflow flags, termination triggers on overflow, non-blocking asynchronous draining, and bounded shutdown drains.
6. **Remediation Register Continuity:** Explicit preservation of original audit findings (**FIND-001** through **FIND-013**) as the overarching project baseline.

---

## 2. Issue 1: Linux Network Egress Verification Protocol

*Technical Reality:* Merely calling `unshare(CLONE_NEWNET)` creates a namespace, but does not guarantee that the container cannot transmit packets if host interfaces (such as `veth` pairs, `macvlan`, or bridge interfaces) are attached, or if an inherited socket is open.

### 2.1 Dynamic Runtime Network Capability Probe
Before candidate execution, the supervisor probes the network namespace state from within the container:
1. **Interface Enumeration (`/sys/class/net` / `netlink`):**
   - The supervisor queries network interfaces via netlink (`RTM_GETLINK`).
   - Invariant: The **only** permitted interface is `lo`. If any external interface (`eth0`, `wlan0`, `veth*`, `tun*`, `tap*`) is present, execution aborts with `SandboxError::NetworkBoundaryCompromised`.
2. **Loopback Operational State:**
   - The loopback interface must **remain unconfigured and DOWN** (`IFF_UP` flag not set).
   - Any attempt to bind to `127.0.0.1` or `::1` fails with `EADDRNOTAVAIL` or `ENETDOWN`.
3. **Routing Table Audit (`RTM_GETROUTE`):**
   - The supervisor queries the container routing table.
   - Invariant: The routing table must be **completely empty** (0 routes). No default gateway may exist.
4. **Inherited Socket Audit:**
   - Atomic `close_range(3, ~0U, 0)` eliminates parent socket inheritance.

---

## 3. Issue 2: PID Namespace Supervision & Thread-Binding Invariants

*Technical Reality:* In Linux, `prctl(PR_SET_PDEATHSIG, ...)` is tied to the **calling thread**, not the parent process. If a multi-threaded Rust supervisor spawns the child from a worker thread that finishes or terminates while the main process survives, the child receives a premature `SIGKILL`. Furthermore, relying solely on PIDs introduces PID-wrap races.

### 3.1 Dedicated Supervisor Thread & `pidfd` Architecture

```
[ Rust Runtime / SECE Supervisor ]
              │
              ▼ Spawns dedicated monitor thread
[ Dedicated Persistent Supervisor Thread (TID T) ]
              │ 1. clone(CLONE_NEWPID | CLONE_NEWUSER | CLONE_NEWNS | CLONE_NEWNET)
              │ 2. Obtains pidfd = pidfd_open(child_pid, 0)
              ▼
[ Child Process (PID 1 in Namespace) ]
              │ Calls prctl(PR_SET_PDEATHSIG, SIGKILL) bound to TID T
              │ Calls prctl(PR_SET_NO_NEW_PRIVS, 1)
              ▼ execve()
[ Candidate Executable ]
```

1. **Dedicated Supervisor Thread:**
   - Process spawning and monitoring are owned by a single, dedicated, long-lived supervisor thread that **never terminates prematurely**.
   - The child's `PR_SET_PDEATHSIG` is bound directly to this persistent thread's TID. If the thread or entire process dies, the child receives `SIGKILL` instantly.
2. **`pidfd` Tracking (Linux 5.3+):**
   - Immediately upon `clone()`, the supervisor obtains an immutable file descriptor representing the process: `pidfd = pidfd_open(child_pid, 0)`.
   - Polling and waiting: The supervisor polls `pidfd` using `epoll` or `poll(POLLIN)`. A `pidfd` becomes readable only when the process has exited, completely eliminating PID wrap-around races.
   - Termination: Process killing uses `pidfd_send_signal(pidfd, SIGKILL, NULL, 0)`.
3. **Double-Kill Invariant:**
   - When terminating, the supervisor sends `SIGKILL` via `pidfd_send_signal`.
   - Because the target is PID 1 of the PID namespace, the Linux kernel propagates `SIGKILL` to all descendants within that namespace.
   - The supervisor loops on `waitpid(-1, &status, WNOHANG)` until `ECHILD` is returned, guaranteeing zero zombie accumulation.

---

## 4. Issue 3: Memory Accounting & Resource Naming Semantics

*Technical Reality:* `memory.max` in Linux Cgroups v2 does not track userland RSS alone. It charges anonymous memory, swapped memory, dirty page cache buffers, and kernel internal allocations (sockets, dentries, page tables) against the cgroup. On Windows, `JOB_OBJECT_LIMIT_JOB_MEMORY` limits committed virtual memory.

### 4.1 Renamed Invariant: `memory_allocation_ceiling`
The capability invariant is formally renamed from `memory_rss_ceiling` to **`memory_allocation_ceiling`**.

### 4.2 Explicit Platform Semantics:
- **Linux Backend (Cgroups v2 `memory.max`):**
  - Enforces a ceiling on total **kernel-charged memory**.
  - Formula: $\text{Charged Memory} = \text{Anon Memory} + \text{Page Cache} + \text{Kernel Memory (slabs, page tables)}$.
  - When the cgroup exceeds `memory.max`, the Linux kernel triggers the in-cgroup OOM killer, terminating the candidate with `SIGKILL` without terminating host processes.
- **Windows Backend (Job Object `JOB_OBJECT_LIMIT_JOB_MEMORY`):**
  - Enforces a ceiling on total **committed private virtual memory** across all processes in the Job.
  - When total committed memory exceeds `JobMemoryLimit`, subsequent `VirtualAlloc` or heap allocations return `NULL` (`ERROR_NOT_ENOUGH_MEMORY`).

---

## 5. Issue 4: Guarded Mode Trust Boundaries & Policy

*Technical Reality:* `Guarded` mode on macOS and Windows provides process-tree tracking and timeouts, but **cannot prevent filesystem reads or network egress**. Calling this a "sandbox" for untrusted code is a security vulnerability.

### 5.1 Formal Trust Policy & Boundaries:

```
┌───────────────────────────────────────────────────────────────────────────┐
│                           TRUST CLASSIFICATION                            │
├─────────────────────────────────────┬─────────────────────────────────────┤
│  TRUSTED DEVELOPER WORKLOADS        │  UNTRUSTED / 3RD-PARTY CANDIDATES   │
│  • Developer's local source code    │  • Speculative optimization tests   │
│  • Self-authored tuning tests       │  • Dependencies from registries     │
│  • Explicit local benchmarks        │  • Unverified code variations       │
├─────────────────────────────────────┼─────────────────────────────────────┤
│  Permitted Mode:                    │  Permitted Mode:                    │
│  EnforcementMode::Guarded           │  EnforcementMode::Strict ONLY       │
│  (macOS / Windows permitted)        │  (macOS / Windows REFUSED)          │
└─────────────────────────────────────┴─────────────────────────────────────┘
```

1. **Strict Mode Invariant (Zero Trust):**
   - For any untrusted, speculative, or third-party candidate code, the toolchain mandates `EnforcementMode::Strict`.
   - On native macOS and Windows, **execution is refused with an explicit error**:
     ```text
     ❌ Security Error: Untrusted candidate execution refused on native {platform}.
        Strict mode requires kernel-enforced filesystem and network isolation.
        Host platform only provides Guarded process tracking.
     ```
2. **Guarded Mode Invariant (Explicit Developer Opt-in):**
   - `EnforcementMode::Guarded` is permitted **only** when the developer explicitly enables it for their own local trusted code (e.g. `syn tune --guarded`).
   - Every guarded run emits an explicit audit log to stderr:
     ```text
     ⚠️ Sandbox Security Warning [Guarded Mode]:
        Filesystem and network isolation are NOT enforced on native {platform}.
        Running candidate with process-group kill and timeout quotas only.
     ```

---

## 6. Issue 5: Hardened Stream Handling & Pipe Bounding

*Technical Reality:* A single `stream.take()` on stdout/stderr causes pipe deadlocks when one stream overflows while the other blocks, fails to notify the supervisor of stream truncation, and leaves child processes wedged in pipe writes.

### 6.1 Independent Budget & Bounded Draining Specification

```rust
pub struct StreamBudgetConfig {
    /// Maximum allowed bytes on stdout (default: 1 MB)
    pub stdout_limit_bytes: usize,
    /// Maximum allowed bytes on stderr (default: 1 MB)
    pub stderr_limit_bytes: usize,
    /// Maximum drain duration after process exit (default: 500 ms)
    pub drain_timeout: Duration,
    /// Whether exceeding the stream budget terminates the process immediately
    pub terminate_on_overflow: bool,
}
```

### 6.2 Supervisor Draining Protocol:
1. **Independent Non-Blocking Drain Threads:**
   - Dedicated worker threads drain stdout and stderr concurrently via `Read::take(budget + 1)`.
2. **Explicit Overflow Detection:**
   - If a stream reads `budget + 1` bytes, an atomic flag `overflow_detected` is set.
   - The thread immediately discards further input and closes the pipe read end.
3. **Immediate Process Termination on Overflow:**
   - If `terminate_on_overflow` is `true` (default in SECE), the supervisor triggers `kill_tree()` as soon as `overflow_detected` is flagged, terminating runaway output generators before disk space is exhausted.
4. **Bounded Shutdown Drain:**
   - When the process exits or is killed, worker threads have a hard **500ms bounded drain window**.
   - If a stream thread does not join within 500ms, the parent closes the pipes and proceeds, preventing supervisor hangs on unclosed child descriptors.

---

## 7. Remediation Register Continuity (FIND-001 through FIND-013)

The focus on Phase 2 SECE Sandboxing must not detach from the comprehensive remediation baseline. The central remediation register ([13-remediation-register.md](file:///Users/Topman/Documents/Projects/aether/remediation_output/13-remediation-register.md)) remains the single source of truth:

| ID | Title | Phase | Current Status | Notes |
| :--- | :--- | :---: | :---: | :--- |
| **FIND-001** | Insecure Package Lockfile Checksum | Phase 1 | **Resolved (Verified)** | 31/31 adversarial tests passing; tagged `phase1-accepted`. |
| **FIND-002** | SECE Pseudo-Sandbox (Unrestricted Host) | Phase 2 | **REOPENED (Under Architecture)**| Deliverables 01, 02, 03A, 03B active. |
| **FIND-003** | Missing Native Runtime Linker Symbol | Phase 4 | **Resolved (Verified)** | `syntra_channel_select` implemented and verified. |
| **FIND-004** | Permanent Native Heap Leaks | Phase 4 | **Resolved (Verified)** | Runtime mark-and-sweep GC active. |
| **FIND-005** | Async/Await Parser Precedence | Phase 3 | **Resolved (Verified)** | Await precedence grammar verified. |
| **FIND-006** | Type Checker Wildcard Bypass | Phase 3 | **Resolved (Verified)** | Typed await inferences enforced. |
| **FIND-007** through **FIND-013** | Standard Library, Self-Hosting, Approval Gate, Warnings | Various | **Resolved (Verified)** | All 118 workspace regression tests passing. |

---

## 8. Conclusion

Deliverable 03B establishes the final, hardened security contract:
- Runtime-verified network unsharing (no external interfaces, empty route table, loopback down).
- Dedicated supervisor thread and `pidfd` lifecycle management (no thread death races).
- Accurate `memory_allocation_ceiling` accounting.
- Zero-trust boundary: Guarded mode restricted to trusted local developer workloads.
- Asynchronous bounded stream draining with overflow termination.

*Deliverable 03B is complete. Ready to proceed to Deliverable 04 (Adversarial Test Specification).*
