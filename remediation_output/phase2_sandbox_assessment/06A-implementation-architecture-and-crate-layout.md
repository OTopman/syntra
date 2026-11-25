# Syntra Phase 2: SECE Sandbox Security Architecture
# Deliverable 06A: Implementation Architecture & Crate Layout Specification

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6 — Implementation Architecture & Crate Layout  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **STAGE 6 ARCHITECTURE SPECIFICATION — READY FOR REVIEW**

---

## 1. Executive Summary & Purpose

This document establishes the definitive implementation-ready Rust crate architecture for `crates/sandbox`. It translates the approved security specifications (Deliverables 01, 02, 03A, 03B, 04, and 05) into a concrete, modular codebase structure while independently challenging and correcting Deliverable 05's kernel-level assumptions.

### Absolute Governance Constraints Maintained:
- **Zero Production Modifications:** No source files under `crates/sandbox/src/` are modified during this architectural stage.
- **Strict Backward Compatibility:** The public `SandboxRunner` API and existing engine interfaces (`ExperimentalCompilationEngine`, `ApprovedStrategy`) remain 100% compatible with existing callers in `syntra_cli` and workspace benchmarks.
- **Remediation Register Alignment:** Directly addresses and prepares the remediation of **FIND-002** ("SECE Pseudo-Sandbox") without regressing **FIND-001** through **FIND-013**.

---

## 2. Kernel-Level Technical Assumptions: Independent Audit & Corrections

Deliverable 05 identified essential kernel requirements, but an implementation-grade architecture requires rigorous examination of edge cases. We establish the following corrections prior to module definition:

### 2.1 Pipe Closure vs. Poll Loop Behaviour
*Deliverable 05 Assumption:* Calling `close(fd)` on a pipe descriptor from another thread immediately interrupts a blocking `poll()` and delivers `POLLHUP`.
*Kernel Reality & Correction:*
- In POSIX.1 and the Linux kernel, calling `close()` on a file descriptor that is actively being waited on by `poll()` / `epoll()` in another thread has **undefined behaviour**. On Linux, it may return `EBADF`, continue blocking until timeout, or suffer from race conditions where the OS assigns that exact descriptor integer to an unrelated file opened by another thread.
*Architectural Rule:* The supervisor **never closes a file descriptor from a separate thread while another thread is polling it**. Instead, all stream polling uses a non-blocking poll loop driven by a **monotonic deadline slice** (e.g. 50 ms slices up to the 500 ms drain budget) or a wake-up self-pipe / eventfd. The polling thread itself performs the final `close()` when it exits the event loop.

### 2.2 Parent Thread ID vs. Process ID in `PR_SET_PDEATHSIG`
*Deliverable 05 Assumption:* Child verifies parent survival with `if (getppid() != supervisor_tid) { _exit(1); }`.
*Kernel Reality & Correction:*
- In Linux, `getppid()` returns the **Thread Group ID (TGID / Process ID)** of the parent, **not the thread ID (TID)** of the specific supervisor thread.
- If the supervisor is spawned as a worker thread in a multi-threaded Rust process, `getppid()` returns the main application PID, which will not match `supervisor_tid`!
- Furthermore, `getppid()` cannot detect if a secondary supervisor thread has exited if the main process remains alive.
*Architectural Rule:* The child process must **not** rely on `getppid() == supervisor_tid`. Instead, parent-child survival is synchronized via a **dedicated unidirectional POSIX pipe (`sync_pipe`) created with `O_CLOEXEC`**. If the supervisor thread dies for any reason, the kernel automatically closes the parent's pipe write end, delivering an instant `EOF` / `EPIPE` to the child, which immediately self-terminates before `execve()`.

### 2.3 `clone3` / `clone` Lifecycle and Synchronization Semantics
*Deliverable 05 Assumption:* Spawning directly calls `clone3(CLONE_PIDFD)` or `clone()`.
*Kernel Reality & Correction:*
- Calling raw `libc::clone()` in a multi-threaded Rust process requires allocating a custom child stack and function pointer, which bypasses standard C runtime initialization.
- Calling `unshare(CLONE_NEWPID)` inside a process does **not** put the calling process into the new PID namespace—it only puts **subsequent children** into the new namespace.
*Architectural Rule:* 
1. The supervisor launches the child via a specialized helper trampoline or `std::os::unix::process::CommandExt::pre_exec`.
2. Inside `pre_exec`, the child calls `unshare(CLONE_NEWUSER | CLONE_NEWNS | CLONE_NEWNET | CLONE_NEWPID)`.
3. To place the candidate into the new PID namespace as PID 1, the trampoline performs an inner `fork()`, where the inner child becomes PID 1 of the isolated namespace, while the intermediary child acts as a namespace supervisor or exits cleanly.
4. Synchronization handshake with the parent ensures namespaces and cgroups are fully configured before the inner child calls `execve()`.

### 2.4 Cgroup Attachment Ordering & Race Conditions
*Deliverable 05 Assumption:* Child PID is written to `cgroup.procs` after `clone()`.
*Kernel Reality & Correction:*
- If the child process begins executing before the parent writes the PID to `cgroup.procs`, the candidate executes unbounded code during that window.
*Architectural Rule:* The child **must block on the synchronization pipe** before calling `execve()`. The parent writes the child's host PID to `/sys/fs/cgroup/.../cgroup.procs`, verifies the write succeeded, sets `memory.max` and `memory.swap.max = 0`, and only then writes a release byte to the synchronization pipe, releasing the child to exec.

### 2.5 Cgroup OOM Telemetry Interpretation
*Deliverable 05 Assumption:* Inspect `oom_kill > 0` in `memory.events`.
*Kernel Reality & Correction:*
- In Cgroups v2, if multiple processes are forked, standard OOM kill may kill one leaf process while leaving others running.
*Architectural Rule:* The Linux provider writes `1` to `memory.oom.group`. This instructs the Linux kernel to atomically terminate **all** processes in the cgroup if an OOM occurs. The supervisor checks `oom_kill > 0 || oom_group_kill > 0` in `memory.events`.

### 2.6 Windows Descendant Containment & Job Objects
*Deliverable 05 Assumption:* Windows Job Objects automatically confine all descendant processes.
*Kernel Reality & Correction:*
- If a child process creates a process with `CREATE_BREAKAWAY_FROM_JOB`, it escapes the Job Object unless the Job is configured with `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` disabled and breakaway explicitly disallowed.
*Architectural Rule:* The Windows provider sets `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` without breakaway permissions, and sets `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, ensuring closing the Job handle terminates all descendants.

---

## 3. Proposed Module Architecture & Hierarchy

The refactored `crates/sandbox` crate will be organized into a clean, decoupled module hierarchy:

```
crates/sandbox/src/
├── lib.rs              # Crate root, re-exports, SECE engine integration
├── error.rs            # Formal SandboxError enum and result types
├── traits.rs           # Core abstract provider and child contracts
├── capabilities.rs     # 4-state capability model and host detection probes
├── config.rs           # Execution modes, resource quotas, stream configs
├── supervisor.rs       # Platform-agnostic process supervision and bounded I/O loop
├── runner.rs           # High-level SandboxRunner facade (backward compatible)
├── candidate.rs        # Candidate representation & transformation passes (existing)
├── verifier.rs         # EquivalenceVerifier differential testing (existing)
├── benchmark.rs        # BenchmarkEvaluator multi-objective metrics (existing)
└── platform/           # Native OS isolation backends
    ├── mod.rs          # Platform selection and fallback dispatch
    ├── linux.rs        # Linux namespaces, cgroups v2, seccomp, pidfd
    ├── macos.rs        # macOS POSIX process groups, rlimits, guarded mode
    ├── windows.rs      # Windows Job Objects, memory/process limits
    └── fallback.rs     # Fail-closed provider for unverified platforms
```

---

## 4. Module Responsibilities & Dependency Boundaries

```
┌────────────────────────────────────────────────────────────────────────┐
│                          crates/sandbox/src/                           │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   ┌────────────────────────────────────────────────────────────────┐   │
│   │               High-Level Engine / Public Facades               │   │
│   │   • lib.rs (ExperimentalCompilationEngine)                     │   │
│   │   • runner.rs (SandboxRunner Facade - Backward Compatible)     │   │
│   └───────────────────────────────┬────────────────────────────────┘   │
│                                   │                                    │
│   ┌───────────────────────────────▼────────────────────────────────┐   │
│   │                     Orchestration Layer                        │   │
│   │   • supervisor.rs (ProcessSupervisor, StreamDrainer)           │   │
│   │   • config.rs (SandboxConfig, EnforcementMode)                 │   │
│   │   • error.rs (SandboxError)                                    │   │
│   └───────────────────────────────┬────────────────────────────────┘   │
│                                   │                                    │
│   ┌───────────────────────────────▼────────────────────────────────┐   │
│   │                      Abstraction Layer                         │   │
│   │   • traits.rs (SandboxProvider, IsolatedChild, PlatformHandle) │   │
│   │   • capabilities.rs (PlatformCapabilities, CapabilityState)    │   │
│   └───────────────────────────────┬────────────────────────────────┘   │
│                                   │                                    │
│   ┌───────────────────────────────▼────────────────────────────────┐   │
│   │                    Platform Backends Layer                     │   │
│   │   • platform/linux.rs   (Namespaces, Cgroups v2, Seccomp)      │   │
│   │   • platform/macos.rs   (Process Groups, Resource Limits)      │   │
│   │   • platform/windows.rs (Job Objects, Memory Limits)           │   │
│   │   • platform/fallback.rs(Fail-Closed Safety Provider)          │   │
│   └────────────────────────────────────────────────────────────────┘   │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### Module Responsibilities:

1. **`error.rs`**:
   - Defines `SandboxError` with strongly typed variants for invariant violations, resource limits, host failures, and configuration errors.
   - Implements `std::error::Error` and conversion helpers to ensure existing callers receiving `String` errors remain unbroken.

2. **`traits.rs`**:
   - `SandboxProvider`: Abstract trait for launching isolated candidates, probing platform capabilities, and performing teardown.
   - `IsolatedChild`: Abstract trait representing an active isolated process tree, exposing `pid()`, `wait()`, `try_wait()`, `kill_tree()`, and stream handles.

3. **`capabilities.rs`**:
   - Implements `CapabilityState` (`Enforced`, `Partial`, `Unavailable`, `Unverified`).
   - Implements `PlatformCapabilities` struct covering 8 discrete isolation primitives.
   - Houses `detect_host_capabilities()`: Performs dynamic probing (checking `/proc/sys/kernel/unprivileged_userns_clone`, cgroups v2 mount, netlink socket access) rather than assuming capabilities based on OS string.

4. **`config.rs`**:
   - `EnforcementMode`: `Strict` (zero trust, untrusted candidates, fail-closed) vs. `Guarded` (trusted local developer workloads).
   - `SandboxConfig`: Aggregate configuration including wall-clock timeout, memory allocation ceiling, CPU limits, PID limits, and stream budgets.
   - `StreamBudgetConfig`: Independent budgets for stdout/stderr (default 1 MB), drain timeout (default 500 ms), and overflow policies.

5. **`supervisor.rs`**:
   - Orchestrates the candidate execution loop.
   - Manages non-blocking I/O polling for stdout and stderr using monotonic timeouts.
   - Enforces hard drain deadlines without risking thread leaks or blocking read deadlocks.
   - Coordinates double-kill process tree teardown and zombie reaping.

6. **`platform/` Backends**:
   - `linux.rs`: Implements containerless Linux isolation via namespaces, cgroups v2, Seccomp-BPF, `close_range`, and `pidfd`.
   - `macos.rs`: Implements macOS process-group confinement (`setpgid`, `kill(-pgid)`), POSIX rlimits, and explicit fail-closed enforcement for Strict mode.
   - `windows.rs`: Implements Windows Job Object creation, process assignment, memory/active process limits, and fail-closed enforcement for Strict mode.
   - `fallback.rs`: Dummy provider that unconditionally fails closed for any unsupported or unverified OS.

---

## 5. Detailed API Contracts & Type Definitions

### 5.1 `error.rs` Contract
```rust
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    /// Strict enforcement requested, but host lacks required capabilities
    StrictInvariantViolation {
        missing_invariant: String,
        platform_state: String,
        remediation_hint: String,
    },
    /// Candidate process exceeded wall-clock timeout
    TimeoutExceeded {
        configured_limit: Duration,
        elapsed: Duration,
    },
    /// Memory allocation ceiling exceeded (Cgroup OOM / Job Object)
    MemoryExceeded {
        ceiling_bytes: usize,
        observed_bytes: Option<usize>,
    },
    /// Active process count exceeded fork bomb threshold
    ProcessLimitExceeded {
        max_pids: u32,
    },
    /// Stream output exceeded allocated byte budget
    StreamOverflow {
        stream_name: &'static str,
        limit_bytes: usize,
    },
    /// Platform primitive failure during setup or teardown
    PlatformSetupFailure(String),
    /// Candidate compilation or linking failure
    CompilationFailure(String),
    /// Process IO failure
    IoFailure(String),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StrictInvariantViolation { missing_invariant, platform_state, remediation_hint } => {
                write!(f, "Sandbox Strict Invariant Violation: {} is {}. {}", missing_invariant, platform_state, remediation_hint)
            }
            Self::TimeoutExceeded { configured_limit, elapsed } => {
                write!(f, "Candidate execution timed out after {:?} (limit: {:?})", elapsed, configured_limit)
            }
            Self::MemoryExceeded { ceiling_bytes, observed_bytes } => {
                write!(f, "Candidate exceeded memory ceiling of {} bytes (observed: {:?})", ceiling_bytes, observed_bytes)
            }
            Self::ProcessLimitExceeded { max_pids } => {
                write!(f, "Candidate exceeded maximum allowed process count ({})", max_pids)
            }
            Self::StreamOverflow { stream_name, limit_bytes } => {
                write!(f, "Candidate {} exceeded output limit of {} bytes", stream_name, limit_bytes)
            }
            Self::PlatformSetupFailure(msg) => write!(f, "Sandbox platform setup failed: {}", msg),
            Self::CompilationFailure(msg) => write!(f, "Candidate compilation failed: {}", msg),
            Self::IoFailure(msg) => write!(f, "Sandbox I/O error: {}", msg),
        }
    }
}

impl std::error::Error for SandboxError {}
```

### 5.2 `traits.rs` Contract
```rust
use crate::config::SandboxConfig;
use crate::capabilities::PlatformCapabilities;
use crate::error::SandboxError;
use std::path::Path;
use std::time::Duration;

pub trait IsolatedChild {
    /// Returns the host PID of the root child process
    fn root_pid(&self) -> u32;

    /// Attempts to poll whether the child has completed
    fn try_wait(&mut self) -> Result<Option<i32>, SandboxError>;

    /// Blocks until child completion or timeout
    fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<i32>, SandboxError>;

    /// Unconditionally terminates the entire process tree
    fn kill_tree(&mut self) -> Result<(), SandboxError>;

    /// Takes the readable non-blocking stdout stream if captured
    fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>>;

    /// Takes the readable non-blocking stderr stream if captured
    fn take_stderr(&mut self) -> Option<Box<dyn std::io::Read + Send>>;
}

pub trait SandboxProvider: Send + Sync {
    /// Returns the dynamically probed capabilities of this host provider
    fn capabilities(&self) -> &PlatformCapabilities;

    /// Spawns an isolated candidate process under the configured constraints
    fn spawn(
        &self,
        binary_path: &Path,
        args: &[String],
        config: &SandboxConfig,
    ) -> Result<Box<dyn IsolatedChild>, SandboxError>;

    /// Cleans up any host resources associated with a run
    fn cleanup_run(&self, run_id: &str) -> Result<(), SandboxError>;
}
```

### 5.3 `config.rs` Contract
```rust
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    /// Mandates verified kernel-enforced filesystem and network isolation.
    /// Fails closed if host platform cannot enforce all invariants.
    Strict,
    /// Cooperative developer mode with process-group kill and timeout limits.
    /// Permitted only for trusted local developer workloads.
    Guarded,
}

#[derive(Debug, Clone)]
pub struct StreamBudgetConfig {
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
    pub drain_timeout: Duration,
    pub terminate_on_overflow: bool,
}

impl Default for StreamBudgetConfig {
    fn default() -> Self {
        Self {
            stdout_limit_bytes: 1024 * 1024, // 1 MB
            stderr_limit_bytes: 1024 * 1024, // 1 MB
            drain_timeout: Duration::from_millis(500),
            terminate_on_overflow: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub run_id: String,
    pub sandbox_base: PathBuf,
    pub enforcement_mode: EnforcementMode,
    pub wall_clock_timeout: Duration,
    pub memory_allocation_ceiling_bytes: usize,
    pub max_pids: u32,
    pub streams: StreamBudgetConfig,
    pub env_vars: Vec<(String, String)>,
}

impl SandboxConfig {
    pub fn new(run_id: String) -> Self {
        let sandbox_base = std::env::temp_dir().join("syntra_sandbox").join(&run_id);
        Self {
            sandbox_base,
            run_id,
            enforcement_mode: EnforcementMode::Strict,
            wall_clock_timeout: Duration::from_secs(5),
            memory_allocation_ceiling_bytes: 128 * 1024 * 1024, // 128 MB
            max_pids: 32,
            streams: StreamBudgetConfig::default(),
            env_vars: Vec::new(),
        }
    }
}
```

---

## 6. Process Ownership, Resource Lifecycle & Cleanup Model

### 6.1 Process Ownership Lifecycle

```
[ SandboxRunner ]
       │
       ▼ creates
[ SandboxConfig ] ──► [ SandboxProvider (Platform-specific) ]
                             │
                             ▼ spawns
                      [ IsolatedChild ]
                             │
                             ▼ supervised by
                      [ ProcessSupervisor ]
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
   [ Poll Event Loop ]               [ Teardown & Reap ]
   • Monotonic slices ≤ 50ms         • Double-kill process tree
   • Non-blocking stream drain       • Harvest zombies via waitpid
   • Overflow & timeout checks       • Drain remaining pipes (≤ 500ms)
                                     • Remove cgroup & sandbox temp dir
```

1. **Spawning Phase:**
   - The provider constructs isolation contexts (cgroup v2 directory, unshared namespaces, or Windows Job Object).
   - Parent establishes the `sync_pipe` with `O_CLOEXEC`.
   - Child process forks; child sets `PR_SET_PDEATHSIG`, detaches controlling terminal (`setsid`), and pauses on `sync_pipe`.
   - Parent registers child PID in cgroup / Job Object and sets resource ceilings (`memory.max`, `memory.swap.max = 0`).
   - Parent releases child via `sync_pipe`; child calls `execve()`.

2. **Supervision Phase:**
   - The `ProcessSupervisor` monitors the `IsolatedChild` without blocking reads.
   - Non-blocking I/O is polled alongside process exit status.
   - Stream buffers enforce the configured capacity ceilings.

3. **Teardown & Cleanup Phase:**
   - On completion, timeout, or error, `kill_tree()` signals the entire namespace PID 1 or Job Object.
   - The supervisor reaps the child to eliminate zombie entries.
   - The cgroup directory `/sys/fs/cgroup/syntra_sandbox/<run_id>` is removed.
   - `SandboxRunner::drop()` ensures the temporary filesystem workspace is recursively deleted.

---

## 7. Backward Compatibility Strategy

The existing codebase contains direct consumers of `SandboxRunner` in `crates/sandbox/src/lib.rs` and `crates/cli/src/lib.rs`. The public API of `SandboxRunner` must remain **100% backward compatible**:

```rust
// runner.rs Adapter Facade:
impl SandboxRunner {
    pub fn new(run_id: String) -> Self;
    pub fn with_timeout(mut self, timeout: Duration) -> Self;
    pub fn with_max_output_bytes(mut self, max: usize) -> Self;
    pub fn with_env_var(mut self, key: String, value: String) -> Self;
    pub fn with_enforcement_mode(mut self, mode: EnforcementMode) -> Self; // NEW BUILDER METHOD
    
    pub fn timeout(&self) -> Duration;
    pub fn max_output_bytes(&self) -> usize;
    pub fn run_id(&self) -> &str;
    pub fn sandbox_base(&self) -> &Path;
    pub fn candidate_dir(&self, candidate_id: &str) -> PathBuf;
    
    pub fn compile_candidate(&self, candidate: &Candidate) -> Result<CompilationOutput, String>;
    pub fn execute_binary(&self, binary_path: &Path, args: &[String]) -> Result<RunOutput, String>;
    pub fn cleanup(&self);
}
```

### Internal Implementation Delegation:
- `execute_binary(&self, ...)` delegates internally to `platform::get_default_provider().spawn(...)` and `ProcessSupervisor::supervise(...)`.
- The returned `RunOutput` struct maintains its exact fields: `stdout: String`, `stderr: String`, `exit_code: Option<i32>`, `duration: Duration`, `timed_out: bool`.
- Errors returned by `execute_binary` continue to implement `Result<RunOutput, String>` by formatting `SandboxError` via its `Display` implementation.

---

## 8. Summary of Deliverable 06A

Deliverable 06A establishes an implementation-ready, modular architecture for `crates/sandbox`:
1. Decouples interface contracts (`traits.rs`), capabilities (`capabilities.rs`), configuration (`config.rs`), and platform implementations (`platform/`).
2. Corrects five kernel assumptions from Deliverable 05 (poll loop close semantics, thread ID vs. PID in `getppid`, `clone`/`unshare` child stack handling, cgroup race windows, and Windows Job breakaway flags).
3. Preserves complete backward compatibility for `SandboxRunner` and engine consumers.

*Deliverable 06A complete. Proceeding to Deliverable 06B.*
