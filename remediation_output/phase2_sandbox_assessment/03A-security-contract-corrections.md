    # Syntra Phase 2: SECE Sandbox Security Architecture
    # Deliverable 03A: Security Contract Corrections & Hardened API Specification

    **Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox  
    **Target Milestone:** Phase 2 Technical Corrections (Review of Deliverable 03)  
    **Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
    **Date:** 2026-11-24  
    **Status:** **CORRECTIVE SPECIFICATION — READY FOR REVIEW**

    ---

    ## 1. Executive Summary & Review Scope

    This corrective specification directly addresses the seven critical security issues and three architectural weaknesses identified during the technical review of Deliverable 03.

    ### Core Architectural Pivot:
    1. **Rejection of Coarse Boolean Capabilities:** Replaces simplistic `bool` fields with four explicit, measurable capability states: `Enforced`, `Partial`, `Unavailable`, and `Unverified`.
    2. **Individual Invariant Conjunction:** Eliminates aggregate capability tiers as a substitute for individual verification. In `Strict` mode, **every mandatory invariant must individually evaluate to `Enforced`**; an aggregate label cannot mask a missing control.
    3. **Deep Kernel Realism:** Corrects theoretical assumptions regarding Linux mount propagation, namespace escapes, `/proc` masking, seccomp bypasses (`io_uring`, inherited sockets), Darwin XNU memory unreliability, and Windows Job Object commit limits.
    4. **Compile-Time Elimination of Unsafe Paths:** Restricts `EnforcementMode::UnsafeDisabled` exclusively to test builds (`#[cfg(any(test, feature = "test-support"))]`), guaranteeing that production compiler binaries cannot execute unconfined code.

    ---

    ## 2. Technical Corrections to Platform Contracts

    ### 2.1 Linux Filesystem Confinement: Beyond `CLONE_NEWNS` and `tmpfs`
    *Review Finding 1:* `CLONE_NEWNS` and a `tmpfs` pivot alone do not guarantee confinement. Mount propagation, `/proc`, `/dev`, device nodes, and inherited file descriptors can leak the host.

    #### Corrected Linux Filesystem Contract:
    1. **Mount Propagation Neutralization:**
    Before pivoting into the new root, the supervisor must recursively mark the mount hierarchy as private:
    `mount(NULL, "/", NULL, MS_REC | MS_PRIVATE, NULL)`.
    This prevents mount/unmount events inside the sandbox from propagating to the host, and blocks host mount events from altering the sandbox.
    2. **Masked and Remounted Virtual Filesystems:**
    - `/proc` must be mounted as a fresh, private instance: `mount("proc", "/proc", "proc", MS_NOSUID | MS_NODEV | MS_NOEXEC, NULL)`. Without a private `/proc`, tools inspect host PIDs, and `/proc/sys` leaks kernel tunables.
    - Read-Only Masking: `/proc/sys`, `/proc/sysrq-trigger`, `/proc/irq`, and `/proc/bus` must be remounted read-only (`MS_RDONLY | MS_REMOUNT | MS_BIND`) or covered with an empty read-only `tmpfs`.
    - `/sys` must not be mounted, or mounted read-only with minimal mock data.
    3. **Device Node Restrictions (`/dev`):**
    - The sandbox must **never** bind-mount the host `/dev`.
    - It must mount an isolated `tmpfs` on `/dev` and populate **only** minimal safe pseudodevices via `mknod` or bind-mount: `/dev/null`, `/dev/zero`, `/dev/urandom`, `/dev/random`.
    - All other device nodes (hard disks, raw memory `/dev/mem`, TTYs) are strictly absent.
    4. **Read-Only System Bind Mounts:**
    Runtime libraries (`/lib`, `/lib64`, `/usr/lib`) are bind-mounted with `MS_BIND | MS_REC | MS_RDONLY | MS_NODEV | MS_NOSUID`.
    5. **No-New-Privileges (`PR_SET_NO_NEW_PRIVS`):**
    Before executing candidate code, the sandbox calls `prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)`, preventing SUID/SGID binaries (like `sudo` or `pkexec`) from gaining elevated privileges even if encountered in mounted paths.

    ---

    ### 2.2 Linux PID Namespace Lifecycle: Reaping Races & Death Signals
    *Review Finding 2:* Killing PID 1 is useful, but lifecycle supervision must verify descendant termination and account for races, reaping, and supervisor failure.

    #### Corrected Process Hierarchy Contract:
    ```
    [ Host Compiler Process ]
            │
            ▼ (spawns outside child namespace)
    [ Supervisor Thread / Monitor ]
            │
            ▼ fork/clone(CLONE_NEWPID | CLONE_NEWUSER | CLONE_NEWNS | CLONE_NEWNET)
    [ Sandbox Init Process (PID 1 inside namespace) ]
            │  Sets PR_SET_PDEATHSIG(SIGKILL)
            │  Drops privileges & sets PR_SET_NO_NEW_PRIVS
            ▼ execve()
    [ Candidate Executable (PID 1 or child of PID 1) ]
    ```

    1. **Supervisor Crash Invariant (`PR_SET_PDEATHSIG`):**
    Immediately after `clone()` / `fork()`, the child calls:
    `prctl(PR_SET_PDEATHSIG, SIGKILL)`.
    If the parent supervisor crashes, is killed (`SIGKILL`), or terminates unexpectedly, the Linux kernel immediately sends `SIGKILL` to the sandbox init process.
    2. **PID 1 Exit Guarantees:**
    In Linux, when PID 1 of a PID namespace terminates, the kernel sends `SIGKILL` to every process in that PID namespace and refuses to accept new processes (`CLONE_NEWPID` lifecycle guarantee).
    3. **Active Reaping Loop & Verification:**
    The supervisor does not simply fire `SIGKILL` and return. It executes a formal termination sequence:
    - Send `SIGKILL` to sandbox PID 1.
    - Loop on `waitpid(-1, &status, WNOHANG)` until `ECHILD` is returned.
    - Poll `/proc/<pid>` to verify the process entry is completely reaped before releasing temporary files.

    ---

    ### 2.3 Network Confinement & Seccomp: Sockets, Egress, and `io_uring`
    *Review Finding 3:* Blocking selected networking syscalls does not prevent every networking path. Must account for inherited sockets, `io_uring`, and Unix domain sockets.

    #### Corrected Network & Syscall Contract:
    1. **Network Namespace Boundary:**
    `CLONE_NEWNET` unshares the network stack. Only an unconfigured `lo` interface exists. Any attempt to route TCP/UDP packets out of the container fails at the kernel routing table (`ENETUNREACH`).
    2. **Inherited Socket Sanitization:**
    Before `execve()`, all inherited file descriptors > 2 are closed (see Section 5). An attacker cannot inherit an open TCP connection or UNIX domain socket from the parent compiler.
    3. **Seccomp Blocking of `io_uring` and Kernel Async Sockets:**
    Modern Linux kernels permit network I/O through `io_uring_enter` (`SYS_io_uring_setup`, `SYS_io_uring_enter`, `SYS_io_uring_register`), which can bypass classic syscall filters.
    - **Mandatory Seccomp Invariant:** The Seccomp filter must **explicitly deny `SYS_io_uring_setup`, `SYS_io_uring_enter`, and `SYS_io_uring_register`** with `SECCOMP_RET_ERRNO(EPERM)`.
    4. **UNIX Domain Socket Confinement:**
    UNIX domain sockets (`AF_UNIX`) require filesystem path binding or abstract namespace access.
    - Filesystem UNIX sockets (e.g. Docker daemon `/var/run/docker.sock`, X11 `/tmp/.X11-unix`) are blocked because the mount namespace does not mount host `/var/run` or `/tmp`.
    - Abstract UNIX sockets (`AF_UNIX` with null-byte prefix) share network namespaces! Because `CLONE_NEWNET` creates a private network namespace, **abstract UNIX domain sockets are completely isolated** from host abstract sockets.

    ---

    ### 2.4 Windows Job Object Memory & Lifecycle Guarantees
    *Review Finding 4:* Job Object memory limits need precise distinction between per-process and job-wide limits, including accounting and enforcement mechanisms.

    #### Corrected Windows Contract:
    1. **Job-Wide vs. Per-Process Memory Ceiling:**
    - Per-process limit (`JOB_OBJECT_LIMIT_PROCESS_MEMORY` via `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`): Restricts individual process virtual memory, but a candidate spawning 10 processes could consume $10 \times \text{limit}$.
    - **Mandatory Syntra Invariant:** Must configure **`JOB_OBJECT_LIMIT_JOB_MEMORY`** in conjunction with `ProcessMemoryLimit`. `JobMemoryLimit` sets a hard committed memory ceiling across the **entire collection of processes** within the Job. When total committed memory exceeds this limit, memory allocation calls (`VirtualAlloc`, heap expansions) fail with `ERROR_NOT_ENOUGH_MEMORY`.
    2. **Termination Invariant (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`):**
    - Configured via `JOBOBJECT_EXTENDED_LIMIT_INFORMATION.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    - When the Rust supervisor drops the Job Object handle (`CloseHandle(hJob)`), the Windows kernel forcibly terminates every process assigned to the Job.
    3. **Limitation Disclosed:** Memory shared across system DLLs is accounted under Windows kernel working set policies; strict private commitment is the audited boundary.

    ---

    ### 2.5 CPU Control Dimensions: Wall-Clock, CPU Time, and CPU Bandwidth
    *Review Finding 5:* CPU time, CPU bandwidth, and wall-clock time are different guarantees. The capability model must distinguish them adequately.

    #### Explicit Tripartite CPU Model:

    | Dimension | Definition | Host Threat Mitigated | Enforcement Mechanism |
    | :--- | :--- | :--- | :--- |
    | **1. Wall-Clock Timeout** | Absolute elapsed duration from spawn to termination. | Hanging on I/O, deadlocks, `sleep()` evasion. | Supervisor polling loop with `Instant::now()` + timer thread. |
    | **2. Cumulative CPU Time** | Total user + system CPU execution time consumed across all threads. | Slow-running computational spin-locks, multithreaded busy-loops. | Linux: Cgroups v2 `cpu.stat` / `RLIMIT_CPU`.<br>Windows: `JOB_OBJECT_LIMIT_JOB_TIME`.<br>macOS: Thread sampling / `RLIMIT_CPU`. |
    | **3. CPU Bandwidth / Throttling** | Maximum CPU core quota allowed per time period (e.g. max 1 core = 100ms per 100ms period). | Starving host processes, saturating all host CPU cores during the timeout window. | Linux: Cgroups v2 `cpu.max` (e.g. `100000 100000`).<br>Windows: Job Object `SetInformationJobObject(JobObjectCpuRateControlInformation)`.<br>macOS: **Unavailable** (Native process cannot throttle cores). |

    ---

    ### 2.6 Hardening `UnsafeDisabled` Against Production Exposure
    *Review Finding 6:* An internal-testing designation is not an enforcement mechanism. Production binaries must not expose unrestricted execution.

    #### Corrected Build Guard:
    ```rust
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EnforcementMode {
        /// Mandatory full isolation. Rejects execution if any invariant fails.
        Strict,
        /// Guarded execution for local developer iteration with explicit audit warnings.
        Guarded,
        /// Internal test harness mode ONLY. Completely omitted from release builds.
        #[cfg(any(test, feature = "test-support"))]
        UnsafeDisabled,
    }
    ```
    - In release toolchain builds (`--release`), `EnforcementMode::UnsafeDisabled` **does not exist** at the type level.
    - Any attempt to configure an unconfined sandbox mode in a production compiler binary produces a compile-time error.

    ---

    ### 2.7 Process Lifecycle & Stream Bounding Contract
    *Review Finding 7:* `wait_timeout`, `try_wait`, and `kill_tree` need clearly defined ownership, timeout, output collection, and cleanup semantics.

    #### Lifecycle Contract Specification:
    1. **Ownership & State Transitions:**
    - Spawning creates an `IsolatedChild` in state `Running`.
    - `try_wait()` queries status without blocking. Returns `Ok(None)` if running, `Ok(Some(ExitStatus))` if reaped.
    - `wait_timeout(timeout)` consumes execution time up to `timeout`. Transitions to `Terminated` or `TimedOut`.
    - `kill_tree()` transitions to `Killed`. It is idempotent (calling it multiple times is safe and returns `Ok(())`).
    2. **Stream Bounding & Pipe Deadlock Prevention:**
    - Stdout and Stderr streams are drained asynchronously in dedicated worker threads via `stream.take(max_output_bytes as u64)`.
    - If candidate emits $> \text{max_output_bytes}$, the reader stops reading, remaining child writes block on the pipe buffer (64KB), and the supervisor terminates the candidate with `SandboxError::OutputLimitExceeded`.
    - Prevents both memory exhaustion in supervisor and pipe buffer deadlocks.
    3. **Workspace Purging:**
    - Ephemeral sandbox directories (`/tmp/syntra_sandbox/<run_id>`) implement RAII cleanup via `Drop`.
    - On drop, the supervisor executes `kill_tree()`, reaps all children, and calls `fs::remove_dir_all`.

    ---

    ## 3. Fine-Grained Capability State Model

    To eliminate vague Boolean declarations, all capabilities are represented using explicit four-state enums:

    ```rust
    /// Concrete enforcement state of an individual security invariant
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CapabilityState {
        /// Guaranteed and enforced by kernel isolation primitives
        Enforced,
        /// Partial or cooperative protection (subject to known bypasses)
        Partial,
        /// The host operating system or environment cannot provide this control
        Unavailable,
        /// Not yet probed or dynamically verified on this host
        Unverified,
    }
    ```

    ### Invariant-by-Invariant Platform Capability Structure:

    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PlatformCapabilities {
        pub platform_name: &'static str,
        
        // Filesystem Invariants
        pub filesystem_mount_confinement: CapabilityState,
        pub filesystem_readonly_system: CapabilityState,
        
        // Network Invariants
        pub network_egress_blocking: CapabilityState,
        pub network_socket_suppression: CapabilityState,
        
        // Process & Tree Invariants
        pub process_tree_immune_kill: CapabilityState,
        pub process_count_limit: CapabilityState,
        
        // Resource Quota Invariants
        pub memory_rss_ceiling: CapabilityState,
        pub cpu_time_limit: CapabilityState,
        pub cpu_bandwidth_throttling: CapabilityState,
        pub wall_clock_timeout: CapabilityState,
        
        // Syscall Invariants
        pub syscall_filtering: CapabilityState,
        
        // Security Hygiene
        pub inherited_fd_sanitization: CapabilityState,
    }
    ```

    ---

    ## 4. Invariant Conjunction Matrix (Strict vs. Guarded)

    A provider is **never** approved based on an aggregate label. Every execution mode evaluates explicit predicate conjunctions over the capability structure:

    ```rust
    impl PlatformCapabilities {
        /// Validates compliance with Strict mode invariants
        pub fn verify_strict_compliance(&self) -> Result<(), Vec<&'static str>> {
            let mut violations = Vec::new();

            if self.filesystem_mount_confinement != CapabilityState::Enforced {
                violations.push("filesystem_mount_confinement (Kernel mount unshare required)");
            }
            if self.network_egress_blocking != CapabilityState::Enforced {
                violations.push("network_egress_blocking (Kernel network unshare required)");
            }
            if self.process_tree_immune_kill != CapabilityState::Enforced {
                violations.push("process_tree_immune_kill (Immune process-tree termination required)");
            }
            if self.memory_rss_ceiling != CapabilityState::Enforced {
                violations.push("memory_rss_ceiling (Kernel memory quota required)");
            }
            if self.wall_clock_timeout != CapabilityState::Enforced {
                violations.push("wall_clock_timeout (Wall-clock timeout required)");
            }
            if self.inherited_fd_sanitization != CapabilityState::Enforced {
                violations.push("inherited_fd_sanitization (File descriptor hygiene required)");
            }

            if violations.is_empty() {
                Ok(())
            } else {
                Err(violations)
            }
        }

        /// Validates compliance with Guarded mode invariants
        pub fn verify_guarded_compliance(&self) -> Result<(), Vec<&'static str>> {
            let mut violations = Vec::new();

            if self.wall_clock_timeout != CapabilityState::Enforced {
                violations.push("wall_clock_timeout");
            }
            if self.inherited_fd_sanitization != CapabilityState::Enforced {
                violations.push("inherited_fd_sanitization");
            }
            if self.process_tree_immune_kill == CapabilityState::Unavailable {
                violations.push("process_tree_immune_kill (Must have at least process-group kill)");
            }

            if violations.is_empty() {
                Ok(())
            } else {
                Err(violations)
            }
        }
    }
    ```

    ### Evaluated Enforcement Matrix:

    | Individual Invariant | Linux (Tier 1) | macOS (Tier 2) | Windows (Tier 2) | Mandatory in Strict? |
    | :--- | :---: | :---: | :---: | :---: |
    | `filesystem_mount_confinement` | **Enforced** | **Unavailable** | **Unavailable** | **YES** |
    | `filesystem_readonly_system` | **Enforced** | **Partial** | **Partial** | Optional |
    | `network_egress_blocking` | **Enforced** | **Unavailable** | **Unavailable** | **YES** |
    | `network_socket_suppression` | **Enforced** (Seccomp) | **Unavailable** | **Unavailable** | Optional |
    | `process_tree_immune_kill` | **Enforced** (PID ns) | **Partial** (`killpg`) | **Enforced** (JobObject) | **YES** |
    | `process_count_limit` | **Enforced** (`pids.max`) | **Partial** (`RLIMIT`) | **Enforced** (`JobLimit`) | Optional |
    | `memory_rss_ceiling` | **Enforced** (cgroups v2) | **Unavailable** | **Enforced** (`JobMemory`) | **YES** |
    | `cpu_time_limit` | **Enforced** | **Partial** | **Enforced** | Optional |
    | `cpu_bandwidth_throttling` | **Enforced** (`cpu.max`) | **Unavailable** | **Enforced** | Optional |
    | `wall_clock_timeout` | **Enforced** | **Enforced** | **Enforced** | **YES** |
    | `syscall_filtering` | **Enforced** (Seccomp) | **Unavailable** | **Unavailable** | Optional |
    | `inherited_fd_sanitization` | **Enforced** (`close_range`) | **Enforced** (`proc/fd`) | **Enforced** (`bInherit=F`) | **YES** |
    | **Strict Mode Result** | **PERMITTED** | **REFUSED** | **REFUSED** | — |
    | **Guarded Mode Result** | **PERMITTED** | **PERMITTED (Warn)**| **PERMITTED (Warn)**| — |

    ---

    ## 5. Inherited Resource Hygiene (File Descriptors, Handles, Sockets)

    A major supply-chain threat vector is the inheritance of open resources across `fork()`/`execve()`. If the parent compiler process has open file handles (e.g. `Package.lock`, database handles, Git repositories, or socket connections), child processes inherit them unless sanitized.

    ### Sanitization Implementation Contract:
    1. **Linux (Kernel 5.9+):**
    Execute `close_range(3, ~0U, 0)`. This atomically closes every file descriptor above standard error with zero traversal overhead.
    2. **POSIX Fallback (macOS / older Linux):**
    - Iterate `/dev/fd` (macOS) or `/proc/self/fd` (Linux) and close all FDs > 2.
    - Fallback: Query `getrlimit(RLIMIT_NOFILE)` and close loop up to max limit.
    3. **Windows:**
    When spawning via `CreateProcessW`:
    - Set `bInheritHandles = FALSE`.
    - Alternatively, configure an explicit empty handle whitelist using `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`.

    ---

    ## 6. Updated Rust Trait & Error Contracts

    ```rust
    pub trait SandboxProvider: Send + Sync {
        /// Queries the dynamically verified security capabilities of the host platform
        fn capabilities(&self) -> PlatformCapabilities;

        /// Spawns an isolated candidate process enforcing the configured limits
        fn spawn(
            &self,
            binary_path: &Path,
            args: &[String],
            config: &SandboxConfig,
        ) -> Result<Box<dyn IsolatedChild>, SandboxError>;
    }

    pub trait IsolatedChild: Send {
        /// Polls execution status without blocking
        fn try_wait(&mut self) -> Result<Option<ExitStatus>, SandboxError>;

        /// Blocks until process exits or timeout occurs, enforcing resource limits
        fn wait_timeout(&mut self, timeout: Duration) -> Result<RunOutput, SandboxError>;

        /// Terminates the entire process tree immediately with SIGKILL
        fn kill_tree(&mut self) -> Result<(), SandboxError>;

        /// Collects execution statistics
        fn execution_metrics(&self) -> Option<ExecutionMetrics>;
    }
    ```

    ### Typed Error Model:
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SandboxError {
        /// Refusal because platform cannot satisfy individual strict-mode invariants
        StrictInvariantViolation {
            unmet_invariants: Vec<&'static str>,
            platform: &'static str,
        },
        /// General capability refusal
        CapabilityRefusal(String),
        /// Candidate exceeded configured time quota
        TimedOut { elapsed: Duration },
        /// Candidate exceeded memory quota
        MemoryExhausted { limit_bytes: u64 },
        /// Candidate exceeded output byte ceiling
        OutputLimitExceeded { limit_bytes: usize },
        /// Process spawning or namespace unsharing failed
        SpawnFailed(String),
        /// Process polling or supervision failed
        SupervisionError(String),
        /// Ephemeral workspace preparation or cleanup error
        WorkspaceError(String),
    }
    ```

    ---

    ## 7. Empirical Adversarial Verification Targets (Replacing Assumptions)

    Before code acceptance, every platform backend must be subjected to real adversarial verification:

    | Attack Vector | Adversarial Test | Target Verification Invariant |
    | :--- | :--- | :--- |
    | **Daemon Escape** | Script executes `setsid()` + background sleep | Asserts `kill_tree()` kills both parent and session leader. |
    | **FD Hijacking** | Parent opens sensitive temp file; child scans FDs 3..1024 | Asserts child cannot read parent FD (returns `EBADF`). |
    | **Procfs Escape** | Candidate reads `/proc/1/environ` and `/proc/sys` | Asserts `/proc/1` is container init, `/proc/sys` is read-only. |
    | **Network Egress** | Candidate connects to `1.1.1.1:53` and `127.0.0.1:80` | Asserts connection returns `ENETUNREACH` or is blocked by filter. |
    | **Memory Bomb** | Candidate calls `malloc` or `mmap` allocating 2GB | Asserts process is killed without host swapping. |
    | **Fork Bomb** | Script executes `:(){ :|:& };:` | Asserts `pids.max` halts fork proliferation in <1s. |
    | **Strict Refusal** | In `Strict` mode on macOS, invoke candidate | Asserts `Err(SandboxError::StrictInvariantViolation)` returned. |

    ---

    ## 8. Conclusion

    Deliverable 03A provides the necessary mathematical and kernel rigor:
    - Replaces vague Boolean capabilities with four-state `CapabilityState` values.
    - Evaluates individual invariants in conjunction, eliminating tier-based masking.
    - Implements `close_range` file descriptor sanitization.
    - Restricts `UnsafeDisabled` to compile-time test builds.
    - Establishes a fail-closed execution model.

    *Deliverable 03A is complete. Production code remains untouched pending review.*
