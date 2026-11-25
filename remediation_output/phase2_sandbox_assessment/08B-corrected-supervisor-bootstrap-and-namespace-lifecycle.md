# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 08B: Corrected Supervisor, Bootstrap & Namespace Lifecycle Architecture

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 Corrective Package (Resolves Findings 1, 3, 6; `RT-01`, `RT-03`, `RT-06`; `IND-02`, `IND-03`, `IND-10`)  
**Date:** 2026-11-24  
**Status:** **CORRECTED ARCHITECTURE SPECIFICATION — SUPERSEDES `06A` & `07B` LIFECYCLE SECTIONS**

---

## 1. Purpose & Responsibility Model

To eliminate parent-death signal confusion, process-wide file descriptor leakage, and untrusted PID 1 privilege defects identified in Stage 6.5 Review Findings 1, 3, and 6, the process lifecycle is governed by a strict **5-Role Responsibility Model**.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                     5-ROLE LIFECYCLE & TRUST HIERARCHY                     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  [Tier 0] Syntra Compiler Host (syn / syntra)                              │
│     • Trust: Trusted host process (multi-threaded)                         │
│     • Role: Candidate generation, orchestration, invokes SandboxRunner     │
│                                                                            │
│       │ spawns dedicated single-threaded supervisor + sync_t0_t1 pipe      │
│       ▼ (Monitored by pidfd_t1)                                            │
│  [Tier 1] Dedicated Sandbox Supervisor Process                             │
│     • Trust: Trusted, sterile single-threaded helper process               │
│     • Role: Pre-creates target cgroup, holds cgroup_fd & pidfd_pid1,       │
│       polls bounded stdout/stderr, executes deterministic teardown         │
│     • Termination bond: PR_SET_PDEATHSIG(SIGKILL) -> Tier 0 + sync_t0_t1   │
│                                                                            │
│       │ clone3(CLONE_INTO_CGROUP | CLONE_PIDFD | CLONE_NEWUSER |           │
│       │        CLONE_NEWNS | CLONE_NEWNET | CLONE_NEWPID)                  │
│       ▼ (Created directly inside target cgroup; NO intermediary fork gap)  │
│  [PID 1] Trusted Namespace Init Process                                    │
│     • Trust: Trusted, minimal static/sterile runtime routine               │
│     • Role: Executes ONLY trusted Rust/C init code; sets PR_SET_PDEATHSIG  │
│       to Tier 1 (validated via sync_t1_p1 startup pipe); reaps ALL         │
│       orphaned descendants; never execve()s candidate code                 │
│                                                                            │
│       │ fork() inside PID namespace + Tier 2 Bootstrap Setup               │
│       ▼                                                                    │
│  [Tier 2 Bootstrap -> PID 2+] Trusted Bootstrap transitioning to Candidate │
│     • Before execve: Trusted Bootstrap installs Seccomp-BPF +              │
│       PR_SET_NO_NEW_PRIVS, sanitizes FDs 0..max, setsid(), mount verification│
│     • execve(candidate): Becomes Untrusted Candidate (PID 2+ in namespace) │
│     • Candidate CANNOT replace PID 1 and CANNOT weaken installed filters   │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Core Security Principles of This Hierarchy:
1. **The Candidate Never Enforces Its Own Containment:** Seccomp, mount isolation, namespaces, descriptor sanitization, and cgroups are configured exclusively by Tier 1, Trusted PID 1, and Tier 2 Trusted Bootstrap prior to `execve(candidate)`.
2. **No Intermediary vs. PID 1 Split Ambiguity:** Tier 1 creates **Trusted PID 1 directly** via `clone3(CLONE_INTO_CGROUP | CLONE_PIDFD | CLONE_NEWPID | CLONE_NEWUSER | CLONE_NEWNS | CLONE_NEWNET)`. There is no intermediate parent between Tier 1 and PID 1, so Tier 1 is the direct creator-thread of PID 1 (`PR_SET_PDEATHSIG` tracks Tier 1 directly).
3. **Untrusted Candidate Is PID 2+:** Candidate code runs as PID 2 inside the namespace. Even if the candidate attempts signal masking or ignores `SIGTERM`, it lacks PID 1 init immunity against host/Tier-1/namespace termination, and Trusted PID 1 guarantees deterministic reaping of all forked candidate descendants.

---

## 2. Three Independent Process-Death Containment Protections (Finding 1 Resolution)

Pipe `EOF` and `PR_SET_PDEATHSIG` each have well-documented Linux kernel caveats (`O_CLOEXEC` acts at `exec`, not `fork`; `PR_SET_PDEATHSIG` tracks creator thread and clears across fork or certain credential transitions). Therefore, lifecycle termination **never depends on a single mechanism**:

| Protection Layer | Kernel Primitive | Scope & Behaviour | Failure Mode Covered |
| :--- | :--- | :--- | :--- |
| **Layer 1: Dedicated Single-Threaded Tier 1 Supervisor** | Separate OS process (`exit_group`), explicit pre-fork FD sweep (`close_range` / `O_CLOEXEC`), `pidfd_open` | Tier 1 has an isolated FD table and is single-threaded (TID == TGID == PID). Its death atomically closes all FDs it owns without interference from Tier 0 compiler worker threads. | Eliminates multi-threaded FD retention bugs (`RED-01`). |
| **Layer 2: Kernel Cgroup Tree Kill** | Cgroups v2 `cgroup.kill` (Linux 5.14+), with documented `cgroup.freeze` + iterative `cgroup.procs` `SIGKILL` loop | `cgroup.kill` signals **every** process and thread in the sandbox cgroup atomically in kernel space, regardless of `setsid()`, `fork()`, double-fork, or PID reuse. | Kills multi-generation descendants even if PID 2+ escapes PGID or tries fork-race evasion. |
| **Layer 3: Direct Creator-Thread `PR_SET_PDEATHSIG` + Sync Handshake + Trusted PID 1 Subreaper** | `prctl(PR_SET_PDEATHSIG, SIGKILL)` + `sync_t1_p1` pipe + `PR_SET_CHILD_SUBREAPER` + PID namespace teardown | Trusted PID 1 is directly `clone3()`-created by single-threaded Tier 1. After `prctl(PR_SET_PDEATHSIG, SIGKILL)`, Trusted PID 1 waits for Tier 1 confirmation on `sync_t1_p1`; if Tier 1 dies before `prctl()`, `sync_t1_p1` closes (`exit_group` of Tier 1) and PID 1 exits immediately. When PID 1 dies, the kernel terminates the entire PID namespace (`SIGKILL` to PID 2+). | Protects against Tier 0 crash, Tier 1 crash, and pre-`prctl()` startup race. |

### Addressing Pipe Descriptor Inheritance Across `fork()`:
- Because `O_CLOEXEC` only closes descriptors during `execve()` and **not** across `fork()`:
  1. Every non-shared pipe created by Tier 0 or Tier 1 sets `FD_CLOEXEC` at creation (`pipe2(..., O_CLOEXEC)`).
  2. Immediately after `clone3()` / `fork()`, **both** Trusted PID 1 and Tier 2 Trusted Bootstrap explicitly close every write end of sync/control pipes they do not own **prior** to spawning PID 2+ or executing `execve()`.
  3. Before `execve(candidate)`, Tier 2 Trusted Bootstrap runs `close_range(3, ~0U, 0)` (falling back to enumerating open FDs in isolated `/proc/self/fd` on kernels < 5.9, verifying none remain > 2), ensuring zero supervisor pipe ends ever reach the candidate.

### Crash Survival Matrix:
1. **If Tier 0 (Compiler Host) crashes (`SIGKILL`):**
   - Kernel sends `SIGKILL` to Tier 1 via `PR_SET_PDEATHSIG` (and closes Tier 0's end of `ctrl_t0_t1` pipe if Tier 1 is polling it).
   - Tier 1's `exit_group()` triggers `PR_SET_PDEATHSIG(SIGKILL)` on Trusted PID 1.
   - Kernel kills PID 1 -> PID namespace teardown kills PID 2+ and all descendants.
2. **If Tier 1 (Supervisor) crashes (`SIGKILL`):**
   - `exit_group()` closes Tier 1's end of `sync_t1_p1` and triggers `PR_SET_PDEATHSIG(SIGKILL)` on Trusted PID 1.
   - PID 1 dies -> PID namespace teardown kills PID 2+ and all descendants.
   - Tier 0 observes `pidfd_t1` readable (`POLLIN`) and `waitid` failure, invokes fallback cgroup cleanup (`cgroup.kill` if accessible) and returns fail-closed error.
3. **If Candidate (PID 2+) double-forks descendants and exits:**
   - Descendants inherit both the PID namespace (cannot escape PID 1's namespace) and the sandbox cgroup v2 (`cgroup.procs`).
   - Trusted PID 1 reaps orphans (`waitid`/`waitpid(-1, WNOHANG)`).
   - At deadline or teardown, Tier 1 writes `1` to `cgroup.kill` (or `cgroup.freeze` + `SIGKILL` loop) AND sends `pidfd_send_signal(pidfd_pid1, SIGKILL)`, then verifies `cgroup.procs` is empty before removing the cgroup.

---

## 3. Formal 10-State Process Lifecycle State Machine (Finding 6 Resolution)

Replacing the ambiguous prose in `07B`, every transition below formally defines: **Initiator**, **Receiver**, **Preconditions**, **Kernel Operation**, **Expected Result**, **Timeout**, **Failure Action**, **Cleanup Owner**, and **Evidence Recorded**.

| State / Transition | Initiator -> Receiver | Preconditions | Kernel Operation | Expected Result | Timeout | Failure Action (Terminal Fail-Closed) | Cleanup Owner | Evidence Recorded |
| :--- | :--- | :--- | :--- | :--- | :---: | :--- | :--- | :--- |
| **S0 -> S1**<br>*Spawn Tier 1 Supervisor* | Tier 0 -> Tier 1 | Strict/Guarded config validated; `ctrl_t0_t1` pipe created (`O_CLOEXEC`) | `posix_spawn` / `Command` invoking sterile supervisor mode + `pidfd_open(t1)` | Tier 1 running cleanly; `PR_SET_PDEATHSIG(SIGKILL)` bound to Tier 0 creator | 500 ms handshake | Kill Tier 1 via `pidfd_send_signal`; abort `spawn` | Tier 0 | `supervisor_pid`, `host_kernel_ver`, start timestamp |
| **S1 -> S2**<br>*Pre-Create & Lock Cgroup* | Tier 1 -> Kernel (cgroupfs) | Cgroup v2 unified hierarchy authorized | Create `/sys/fs/cgroup/syntra_sandbox/<run_id>`; write `memory.max`, `memory.swap.max=0`, `memory.oom.group=1`, `pids.max`, `cpu.max`; open `O_PATH` `cgroup_fd` | Cgroup configured before any sandbox child exists | 100 ms (fs ops) | Remove cgroup dir; exit Tier 1 with `PlatformSetupFailure` | Tier 1 | Cgroup path, written limit values, `cgroup_fd` |
| **S2 -> S3**<br>*Create Trusted PID 1 in Cgroup & Namespaces* | Tier 1 -> Trusted PID 1 | `cgroup_fd` open; user/mount/net/pid ns authorized; `sync_t1_p1` & `probe_pipe` created | `clone3()` with `CLONE_INTO_CGROUP \| CLONE_PIDFD \| CLONE_NEWUSER \| CLONE_NEWNS \| CLONE_NEWNET \| CLONE_NEWPID` | Trusted PID 1 born inside target cgroup and new namespaces; Tier 1 holds `pidfd_pid1` | 500 ms | Write `cgroup.kill`; reap; exit fail-closed | Tier 1 | `pidfd_pid1` valid, `/proc/<p1_host_pid>/cgroup` matches |
| **S3 -> S4**<br>*Bind PID 1 Lifecycle & UID/GID Maps* | Trusted PID 1 <-> Tier 1 | `pidfd_pid1` obtained; `sync_t1_p1` held | PID 1 calls `prctl(PR_SET_PDEATHSIG, SIGKILL)` + `prctl(PR_SET_CHILD_SUBREAPER, 1)`; Tier 1 writes `/proc/<p1>/uid_map`, `setgroups=deny`, `gid_map`, confirms handshake on `sync_t1_p1` | PID 1 mapped to isolated UID/GID 0 inside userns; death bond verified | 500 ms | Tier 1 writes `cgroup.kill` + `pidfd_send_signal`; PID 1 self-exits on EOF | Tier 1 | Userns map status, handshake ack |
| **S4 -> S5**<br>*Construct Isolated Rootfs, Net & Virtual FS* | Trusted PID 1 -> Kernel (VFS/Net) | Userns mapped; netns unshared | `mount(MS_REC\|MS_PRIVATE)`; build allowlisted tmpfs root (quota-bounded); `pivot_root`; mount private `/proc` + minimal `/dev`; audit netlink (`RTM_GETLINK`, `RTM_GETROUTE`, loopback DOWN) | Host filesystem unreachable; network down with 0 routes | 500 ms | PID 1 exits non-zero -> namespace teardown; Tier 1 invokes `cgroup.kill` | Tier 1 & Kernel NS | Mount info, netlink probe evidence record |
| **S5 -> S6**<br>*Spawn Tier 2 Bootstrap (PID 2)* | Trusted PID 1 -> Tier 2 Bootstrap | Isolation rootfs & netns verified | PID 1 `fork()`s Tier 2 Bootstrap inside PID namespace; PID 1 remains as dedicated reaper | Tier 2 PID == 2 in namespace; inherited target cgroup | 200 ms | PID 1 kills PID 2, runs `waitpid`, exits fail-closed | Trusted PID 1 / Tier 1 | Candidate namespace PID ($\ge 2$), bootstrap spawn log |
| **S6 -> S7**<br>*Install Mandatory Security Policy* | Tier 2 Bootstrap -> Kernel | No candidate instructions executed yet | Close non-stdio pipe ends; redirect FD 0 -> `/dev/null`, FDs 1/2 -> sandbox pipes; `setsid()`; `RLIMIT_NOFILE`/`RLIMIT_FSIZE`; `prctl(PR_SET_NO_NEW_PRIVS, 1)`; install Seccomp-BPF filter; `close_range(3, ~0U, 0)` | Filters & rlimits permanently locked before `execve`; FD table strictly `[0,1,2]` | 200 ms | `_exit(126)` immediately; PID 1 reports setup failure on `probe_pipe` | Trusted PID 1 / Tier 1 | Seccomp filter load status, open FD count == 3 |
| **S7 -> S8**<br>*Candidate Execution* | Tier 2 Bootstrap -> Candidate (`execve`) | All S0–S7 evidence recorded & Strict conjunction satisfied | `execve(candidate_binary, clean_args, clean_env)` | Untrusted candidate executes as PID 2 with immutable confinement | Wall-clock deadline (`SandboxConfig`) | On `execve` error: `_exit(127)`. On timeout/overflow: Tier 1 triggers teardown | Tier 1 Supervisor | Start monotonic timestamp, exec status |
| **S8 -> S9**<br>*Supervised Monitoring & Bounded Drain* | Tier 1 Supervisor <-> Candidate | Monotonic deadline running; `stdout`/`stderr` FDs `O_NONBLOCK` | `poll()` on `[pidfd_pid1, stdout_fd, stderr_fd, ctrl_t0_t1]` in bounded slices; enforce `stdout_limit_bytes` and `stderr_limit_bytes` separately | Bounded output captured; exit status or violation recorded | `wall_clock_timeout` + 500 ms drain cap | On overflow/timeout/Tier-0 death: transition immediately to S9 teardown | Tier 1 Supervisor | Captured byte counts, `overflow_detected`, elapsed time |
| **S9 -> S10**<br>*Atomic Teardown & Verification* | Tier 1 Supervisor -> Cgroup/NS | Execution ended, timed out, overflowed, or failed | Write `1` to `cgroup.kill` (or freeze+SIGKILL loop); `pidfd_send_signal(pidfd_pid1, SIGKILL)`; PID 1 reaps PID 2+; wait `pidfd_pid1`; verify `cgroup.procs` empty; inspect `memory.events`; remove cgroup dir; delete temp workdir | 0 surviving sandbox processes; cgroup removed; final evidence sealed | 500 ms drain + teardown cap | Retry kill loop; if cgroup still non-empty, mark cleanup failure explicitly (never mask failure!) | Tier 1 & Tier 0 (`Drop`) | `memory.events` (`oom`, `oom_kill`, `oom_group_kill`), empty `cgroup.procs` check, cleanup status |

### Legacy / Older Linux Kernel Clause (`< 5.7` lacking `CLONE_INTO_CGROUP`):
If runtime discovery shows `clone3(CLONE_INTO_CGROUP)` returns `ENOSYS`:
- For **`Strict` mode** requiring guaranteed pre-exec zero-CoW-leakage memory accounting: **Do NOT silently fall back to post-fork `cgroup.procs` migration from a heavy process.** Instead, Tier 1 must already be a sterile minimal binary (`< 4 MB` RSS) placed into the target cgroup *after `execve` of Tier 1* and *prior to* `clone()` of PID 1, OR report `memory_allocation_ceiling` initial-charge verification as `PartiallyEnforced` (triggering fail-closed refusal in Strict mode if exact initial charge isolation cannot be proved). Both paths are explicitly specified and verified in `08C` and `08E`.

---

## 4. Compatibility with `crates/sandbox/src/runner.rs`

Tier 0 (`SandboxRunner`) exposes the exact existing public methods (`new`, `with_timeout`, `with_max_output_bytes`, `with_env_var`, `execute_binary`, `compile_candidate`, `cleanup`, `Drop`), delegating `execute_binary` internally to Tier 1 supervisor orchestration on Linux, and platform-appropriate providers on macOS/Windows.
