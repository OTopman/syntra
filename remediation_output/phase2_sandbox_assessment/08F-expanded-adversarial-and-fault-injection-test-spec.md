# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 08F: Expanded Adversarial & Fault-Injection Test Specification

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 Corrective Package (Resolves Finding 8; `RT-08`, `RT-10`; `IND-07`)  
**Date:** 2026-11-24  
**Status:** **EXPANDED VERIFICATION SPECIFICATION — SUPERSEDES `04` & `07D` TEST SCOPE**

---

## 1. Test Governance Rule: Continuous Co-Implementation Verification

Adversarial and fault-injection tests **shall not be postponed until Stage 8**:
1. Every mechanism implemented during Stage 7 must be paired with its corresponding `ADV-*` and `FLT-*` tests immediately in the same milestone.
2. Tests remain un-ignored only when platform prerequisites exist or explicitly validate platform fail-closed behavior (`ADV-09A`/`ADV-09B`); on non-Linux hosts, Linux-specific kernel probes must NOT silently report `PASS` as host isolation—they must either execute inside an explicit Linux verification environment/CI runner or clearly report `NotApplicable` / platform-skipped in evidence metadata while `ADV-09A`/`09B` strictly verify native fail-closed & Guarded behavior.
3. Every executed test emits a machine-readable Verification Record matching the Deliverable `08E` JSON schema.

---

## 2. Expanded Adversarial Attack Suite (`ADV-01` through `ADV-10`)

Every test specifies all 7 mandatory columns: **Attack Objective**, **Preconditions**, **Attack Procedure**, **Expected Enforcement**, **Expected Evidence**, **Failure Condition**, and **Cleanup Verification**.

### 2.1 Process-Tree & Containment (`ADV-01`)
| Test ID | Attack Objective | Preconditions | Attack Procedure | Expected Enforcement | Expected Evidence | Failure Condition | Cleanup Verification |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **ADV-01A** | Escape via double-fork + `setsid()` | Linux Tier 1+2 Strict | Child forks -> `setsid()` -> forks grandchild writing marker heartbeat to temp file -> child exits immediately | PID ns + Trusted PID 1 + `cgroup.kill` | Sandbox terminates on timeout/exit; `kill(grandchild_pid, 0) == ESRCH` | Grandchild heartbeat updates after teardown | `cgroup.procs` empty; cgroup removed |
| **ADV-01B** | Survive parent-of-candidate termination | Linux Strict | Candidate spawns child, kills its own intermediate parent inside container while descendant sleeps | Trusted PID 1 adopts orphan inside PID ns; `cgroup.kill` kills all on teardown | Descendant reaped; 0 processes remain in cgroup | Descendant reparented to host PID 1 | `/proc` host scan shows 0 markers; `cgroup.procs` empty |
| **ADV-01C** | Multi-generation fork chain (4 levels deep) | Linux Strict | Candidate spawns 4-generation tree before wall-clock timeout | `cgroup.kill` + PID ns teardown | All 4 generations receive `SIGKILL`; exit via timeout | Any generation survives post-run | `cgroup.procs` empty; cgroup dir removed |

### 2.2 Filesystem, Network & Credential Isolation (`ADV-02`, `ADV-03`, `ADV-10`)
| Test ID | Attack Objective | Preconditions | Attack Procedure | Expected Enforcement | Expected Evidence | Failure Condition | Cleanup Verification |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **ADV-02A** | Read host secrets (`/etc/shadow`, `~/.ssh`, `.env`) | Linux Strict | Candidate attempts `open`/`read` on host sensitive paths + compiler workspace files | `pivot_root` allowlist fs (files absent) | All `open` calls fail (`ENOENT`/`EACCES`); 0 secret bytes leaked | Any byte of host secret read | Temp rootfs unmounted and deleted |
| **ADV-02B** | Write outside `/tmp` / exceed tmpfs quota | Linux Strict | Candidate attempts writing `/candidate`, `/lib64`, and writing $> 32\text{ MB}$ to `/tmp` | Read-only rootfs + `tmpfs size=` limit + `RLIMIT_FSIZE` | Root writes fail (`EROFS`/`EACCES`); `/tmp` fill fails (`ENOSPC`/`EFBIG`) | Host disk affected or RO mount modified | Host filesystem free space & checksums intact |
| **ADV-03A** | Outbound UDP/TCP & loopback socket connect | Linux Strict + host local TCP listener on `127.0.0.1` | Candidate opens sockets to `8.8.8.8:53`, `1.1.1.1:80`, and host loopback port | Seccomp `socket` filter (`EPERM`) + `CLONE_NEWNET` (`ENETUNREACH`/`ENETDOWN`) | Socket/connect fails; host loopback listener records 0 connections & 0 bytes | Any packet/connection reaches host listener | Listener stopped cleanly; no netns leak |
| **ADV-10** | Tamper `/proc/sys` or inspect host PIDs in `/proc` | Linux Strict | Candidate lists `/proc/[0-9]*` and attempts write to `/proc/sys/kernel/core_pattern` | Container-private `/proc` + RO `/proc/sys` | Only container PIDs (`1, 2..`) visible; `/proc/sys` write fails (`EROFS`/`EACCES`) | Host PIDs enumerated or `/proc/sys` altered | `/proc` unmounted cleanly with netns/mntns |

### 2.3 Memory, Swap & Accounting (`ADV-04`, Resolving Finding 2)
| Test ID | Attack Objective | Preconditions | Attack Procedure | Expected Enforcement | Expected Evidence | Failure Condition | Cleanup Verification |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **ADV-04A** | Anonymous heap allocation bomb | Linux Strict (`memory.max = 64M`, `swap.max = 0`) | `malloc` + `memset` dirty loop targeting 512 MB | Cgroups v2 `memory.max` + OOM killer | Terminated with `MemoryExceeded`; `memory.events` `oom_kill > 0` | Candidate finishes 512 MB or host OOMs | Cgroup removed; host memory unaffected |
| **ADV-04B** | File-backed `mmap` page-cache exhaustion | Linux Strict (`memory.max = 64M`, `/tmp` bounded) | Create `/tmp/blob` and `mmap(MAP_SHARED \| PROT_WRITE)` + touch pages | Tmpfs size cap + cgroup page-cache charging | Fails with `ENOSPC` / `SIGBUS` / cgroup OOM kill before exceeding limits | Host RAM/cache pressure escapes ceiling | `/tmp` tmpfs destroyed on ns teardown |
| **ADV-04C** | Shared memory (`shm_open` / `memfd_create`) bypass | Linux Strict (`memory.max = 64M`) | Candidate creates `memfd_create`, `ftruncate(256M)`, `mmap`, dirties pages | Cgroups v2 shmem accounting charges `memory.max` | Terminated by cgroup OOM before 256 MB | 256 MB shmem succeeds inside sandbox | All `memfd` mappings reclaimed on process death |
| **ADV-04D** | **Memory-heavy Tier 0 parent CoW inheritance (`Finding 2` Required Test)** | Tier 0 allocates & dirties 256 MB anon pages before calling `SandboxRunner` | Launch candidate under `memory.max = 64M` that allocates 48 MB + forks child dirtying 32 MB | Sterile Tier 1 + `CLONE_INTO_CGROUP` ensures zero Tier 0 CoW charge leakage and enforces 64 MB limit strictly on candidate tree | Sandbox hits 64 MB OOM (`oom_kill > 0`) while Tier 0's 256 MB heap remains completely intact | Sandbox fails immediately from Tier 0 memory OR bypasses 64 MB limit | Tier 0 still holds 256 MB; sandbox cgroup empty & removed |
| **ADV-04E** | Swap exhaustion evasion | Linux Strict (`memory.swap.max = 0`) | Allocate 64 MB + trigger memory churn attempting swap paging | `memory.swap.max = 0` disables swap | Kernel OOM triggers strictly at RAM limit with `memory.swap.current == 0` | Any pages swapped to host disk | `/proc/swaps` usage unaffected |
| **ADV-04F** | Kernel memory / slab pressure | Linux Strict (`memory.max = 64M`, `RLIMIT_NOFILE`) | Rapid pipe/dentry creation in `/tmp` | Cgroup kmem accounting + `nr_inodes` + `RLIMIT_NOFILE` | Hits `EMFILE`/`ENOSPC`/OOM strictly within bounds | Host slab exhaustion | Cgroup & tmpfs torn down |
| **ADV-04G** | Concurrent multi-process memory race | Linux Strict (`memory.max = 64M`, `pids.max = 16`) | Fork 8 children each allocating 16 MB simultaneously (128 MB total) | Aggregate `memory.max` + `memory.oom.group = 1` | Entire cgroup killed atomically (`oom_group_kill > 0`) | Sum exceeds ceiling or partial processes survive | `cgroup.procs` empty; cgroup removed |

### 2.4 Process, FD, Syscall & Stream Abuse (`ADV-05` through `ADV-09`)
| Test ID | Attack Objective | Preconditions | Attack Procedure | Expected Enforcement | Expected Evidence | Failure Condition | Cleanup Verification |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **ADV-05** | Fork bomb (`pids.max`) | Linux Strict (`pids.max = 16`) | Tight `fork()` bomb loop | Cgroups v2 `pids.max` | `fork()` fails (`EAGAIN`); `pids.events: max > 0`; terminated within wall clock | Host PID limit impacted | `cgroup.procs` empty |
| **ADV-06A** | Inherited host TCP/Unix socket FD | Parent opens sensitive Unix/TCP socket FD without manual close | Candidate scans FDs `3..1024` and tries `getsockopt`/`write` | `O_CLOEXEC` + pre-exec FD close + `close_range(3, ~0U, 0)` | All probes fail `EBADF`; parent socket receives 0 bytes | Any probe succeeds or data leaks | Parent closes test socket cleanly |
| **ADV-06B** | Inherited directory FD (`openat` host root escape) | Parent opens `/` directory FD prior to spawn | Candidate calls `openat(fd, "etc/passwd", O_RDONLY)` on inherited FDs | FD purge before `execve` | `openat` fails `EBADF` | Host path opened via directory FD | Host dirfd closed |
| **ADV-06C** | Inherited `memfd_create` / pipe / dup'd bootstrap FDs | Parent opens `memfd` & bootstrap pipes | Candidate reads FDs `3..64` searching for bootstrap pipes or `memfd` secrets | Tier 1/2 closes non-owned pipe ends before fork + `close_range` before `execve` | Zero secret bytes readable; `EBADF` returned | Any bootstrap pipe or `memfd` accessible | Zero leaked FDs |
| **ADV-07** | `io_uring` evasion (`setup`/`enter`/`register`) | Linux Strict | Candidate invokes `syscall(SYS_io_uring_setup/enter/register)` | Seccomp-BPF filter (`EPERM`) or kernel `ENOSYS` | All 3 syscalls fail (`< 0`, `EPERM` or `ENOSYS`); no ring FD created | `io_uring_setup` returns valid FD $\ge 0$ | Process reaped; cgroup removed |
| **ADV-08A** | Simultaneous stdout + stderr 100 MB flood | Any tier (`stdout=64KB`, `stderr=64KB`) | Candidate interleaves writes to FD 1 and FD 2 continuously | Non-blocking `poll()` loop + independent stream budgets | Both outputs strictly $\le 64\text{ KB}$; `StreamOverflow` triggers `kill_tree` | Buffer exceeds limit or supervisor deadlocks | Pipes closed by polling thread; process dead |
| **ADV-08B** | Hung descendant holding stdout/stderr open | Any tier | Candidate spawns sleeper holding FD 1/2 open while writing 0 bytes, then times out | Non-blocking monotonic poll + 500 ms drain cap + `cgroup.kill`/`KILL_ON_JOB_CLOSE` | Supervisor finishes within `timeout + 500ms`; zero hung threads | Supervisor hangs waiting for pipe EOF | All processes killed & descriptors closed |
| **ADV-09A** | Strict mode fail-closed on unsupported host | macOS / Windows / Linux without unprivileged userns | Invoke `EnforcementMode::Strict` | Capability conjunction evaluator | Returns `Err(StrictInvariantViolation)`; 0 processes spawned | Candidate runs on unsupported host | No stray temp dirs or child PIDs |
| **ADV-09B** | Guarded mode honest reporting & teardown (macOS/Win) | macOS or Windows host (`Guarded`) | Run benign + runaway timeout script under `EnforcementMode::Guarded` | Explicit stderr security warning + Job/PGID teardown + dir cleanup | Security limitation banner emitted; runaway process terminated; temp dir deleted | Missing warning OR leaked process/temp dir | Temp dir absent; test child terminated |

---

## 3. Infrastructure Fault-Injection Test Suite (`FLT-01` through `FLT-10`, `IND-07`)

Testing only malicious candidates leaves the sandbox's own failure paths unverified. Every fault-injection test forces failure inside trusted sandbox infrastructure:

| Fault ID | Fault Injected | Expected Fail-Closed Behaviour | Verified Cleanup & Evidence |
| :--- | :--- | :--- | :--- |
| **FLT-01** | Send `SIGKILL` to Tier 0 compiler while Tier 1 + Candidate (with 2-gen descendants) are running | Tier 1 observes Tier 0 death (`PR_SET_PDEATHSIG`/`ctrl_t0_t1` EOF) -> executes `cgroup.kill` -> Trusted PID 1 dies -> PID 2+ & descendants die | 0 candidate processes survive; host test harness checks process markers |
| **FLT-02** | Send `SIGKILL` to Tier 1 supervisor mid-startup (`S3`) and mid-run (`S8`) | Tier 1 `exit_group()` triggers `PR_SET_PDEATHSIG(SIGKILL)` on Trusted PID 1 & closes `sync_t1_p1` -> PID 1 & namespace die immediately | Tier 0 receives supervisor failure, executes emergency `cgroup.kill` + removes cgroup |
| **FLT-03** | Send `SIGKILL` to Trusted PID 1 inside PID namespace | Kernel immediately sends `SIGKILL` to every process in PID namespace (PID 2+) | Tier 1 observes `pidfd_pid1` exit, cleans cgroup, reports unexpected termination |
| **FLT-04** | Simulate failure creating cgroup or `clone3(CLONE_INTO_CGROUP)` error | Tier 1 aborts immediately at `S2`/`S3`; zero candidate instructions executed | Any partially created cgroup directory deleted; returns `PlatformSetupFailure` |
| **FLT-05** | Simulate cgroup `rmdir()` returning `EBUSY` during teardown | Supervisor retries `cgroup.kill` + reap loop; if still non-empty, surfaces explicit teardown error (never swallows failure silently) | Logged explicitly in Verification Record |
| **FLT-06** | Launch 8 concurrent Strict sandboxes simultaneously with unique `run_id`s | Separate cgroups, separate netns/mntns/pidns, no cross-container visibility or FD cross-contamination | All 8 cgroups and temp dirs cleanly removed post-run |
| **FLT-07** | Force Seccomp filter load failure in Tier 2 Bootstrap | Tier 2 aborts (`_exit(126)`) BEFORE `execve(candidate)`; PID 1 signals failure to Tier 1 | Candidate never runs unfiltered; returns fail-closed error |
| **FLT-08** | Pass symlink path as candidate binary or workspace dir pointing outside rootfs | Pre-mount `O_NOFOLLOW` / `RESOLVE_BENEATH` validation rejects symlink traversal | Fails closed before spawning |
| **FLT-09** | Exhaust `/tmp` tmpfs bytes and inodes inside candidate | Writes fail `ENOSPC` inside isolated tmpfs; host `/tmp` remains unaffected | Tmpfs destroyed automatically when mount namespace tears down |
| **FLT-10** | Kill Tier 0 while 4 concurrent sandboxes are active | Every active Tier 1 supervisor terminates its cgroup & Trusted PID 1 tree independently | All 4 sandbox trees terminated; 0 survivor PIDs across host |
