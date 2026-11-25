# Syntra Phase 2: SECE Sandbox Security Architecture
# Deliverable 03: Corrected Platform Capability Matrix & Rust Crate API Design

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox  
**Target Milestone:** Phase 2 Architecture & Interface Specification  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **SPECIFICATION COMPLETE — READY FOR REVIEW**

---

## 1. Executive Summary & Purpose

Deliverable 01 proved that the existing `crates/sandbox` is an execution wrapper rather than a security sandbox. Deliverable 02 established the threat model and core principles.

This document resolves the critical technical assumptions identified during peer review and establishes the formal **Platform Capability Matrix** and **Rust Crate API Design** for `crates/sandbox`.

### Core Tenet: Honest Capabilities & Fail-Closed Enforcement
A sandbox must never promise guarantees that the underlying operating system cannot enforce. If a platform cannot provide the mandated security invariants, Syntra must **programmatically report the missing capabilities and fail closed**.

---

## 2. Technical Corrections to Platform Assumptions

In direct response to architectural review, the following technical corrections are formally incorporated into Syntra's security model:

### 2.1 Linux Networking: Connectivity vs. Socket Creation
- **Correction:** An unconfigured network namespace (`CLONE_NEWNET`) prevents all network transmission and routing (`ENETUNREACH` / unreachable destinations), but the kernel will still allow `socket(AF_INET, ...)` syscalls to create file descriptors.
- **Syntra Contract:** The network guarantee is defined as **Network Connectivity & Exfiltration Isolation** (no packets reach any external network interface, DNS, or loopback host services). Complete suppression of `socket()` creation requires Seccomp syscall filtering.

### 2.2 Linux PID Namespace & Lifecycle Supervision
- **Correction:** Entering a PID namespace requires careful process hierarchy setup. Calling `unshare(CLONE_NEWPID)` affects child processes, not the caller. The first child becomes PID 1 of the new namespace.
- **Syntra Contract:** The supervisor process stays **outside** the child PID namespace. It spawns an intermediary child via `clone(CLONE_NEWPID | CLONE_NEWUSER | CLONE_NEWNS | ...)`, which becomes PID 1 of the container. When the supervisor terminates PID 1, the Linux kernel automatically cleans up and reaps all descendant processes inside that namespace.

### 2.3 macOS Process Groups: The `setsid()` Escape Limitation
- **Correction:** POSIX process groups do **not** provide full process containment. A hostile candidate binary can invoke `setsid()` or `setpgid()` to become a new session leader, detaching from the process group and evading `kill(-pgid, SIGKILL)`.
- **Syntra Contract:** Process group isolation on macOS is formally classified as **Partial (Cooperative/Accidental Escape Resistance)**. It does not provide cryptographic process-tree containment against malicious evasion. In `Strict` mode, execution must be refused.

### 2.4 macOS Memory Limits: `RLIMIT_AS` Unreliability
- **Correction:** On Darwin / macOS (XNU kernel), `RLIMIT_AS` (address space limit) is historically not enforced for memory allocations via `mmap()` or Mach VM calls. Relying on `setrlimit(RLIMIT_AS)` produces a false sense of security.
- **Syntra Contract:** macOS native execution cannot guarantee hard memory ceilings without Hypervisor virtualization. This is recorded as `MemoryLimit::Unsupported` on native Darwin.

### 2.5 macOS Filesystem: Ephemeral Directory vs. Confinement
- **Correction:** Redirecting `TMPDIR` and providing an ephemeral folder does not prevent a candidate from reading `/etc/passwd`, `/Users/...`, or other host files accessible to the user's UID.
- **Syntra Contract:** Filesystem isolation on native macOS is formally classified as **None (Unconfined Host Access)**.

### 2.6 Windows Restricted Tokens: Scope & Limitations
- **Correction:** Restricted tokens strip privileges and disable SIDs, but do not provide network virtualization or filesystem unsharing without AppContainer or Windows Sandbox.
- **Syntra Contract:** Windows native execution provides process and memory containment via Job Objects, but filesystem and network isolation are classified as **Unavailable** on raw Win32.

### 2.7 Seccomp-BPF Syscall Filtering
- **Correction:** A simplistic syscall whitelist breaks legitimate compiler runtimes (Cranelift runtime routines, memory allocators, thread pools, and dynamic linkers need `mprotect`, `futex`, `clone3`, `rt_sigaction`).
- **Syntra Contract:** Seccomp architecture uses a **Categorical Filter**:
  - Explicitly Denied: `ptrace`, `process_vm_writev`, `mount`, `chroot`, `reboot`, `bpf`, `keyctl`, `init_module`.
  - Network Denied in Network-Isolated Mode: `socket`, `connect`, `bind`, `sendto`, `recvfrom`.
  - Permitted: Memory management, synchronization, file I/O within the sandbox file descriptor scope, process termination.

---

## 3. Corrected Platform Capability Matrix

| Guarantee / Invariant | Linux (Tier 1: Kernel Containerless) | macOS (Tier 2: Host Process Guard) | Windows (Tier 2: Job Object Guard) |
| :--- | :--- | :--- | :--- |
| **Filesystem Isolation** | **Full** (`CLONE_NEWNS` + minimal `tmpfs` pivot) | **None** (Host filesystem accessible) | **None** (Host filesystem accessible) |
| **Network Isolation** | **Full Connectivity Block** (`CLONE_NEWNET`) | **None** (Host network accessible) | **None** (Host network accessible) |
| **Process Tree Kill** | **Guaranteed** (`CLONE_NEWPID` kernel cleanup) | **Partial** (Process group; `setsid()` escapes) | **Guaranteed** (`KILL_ON_JOB_CLOSE`) |
| **Memory Ceiling** | **Guaranteed** (Cgroups v2 `memory.max`) | **Unreliable** (`RLIMIT_AS` ignored on Darwin)| **Guaranteed** (`JobMemoryLimit`) |
| **Process Count Limit** | **Guaranteed** (Cgroups v2 `pids.max`) | **Partial** (`RLIMIT_NPROC` per-UID) | **Guaranteed** (`ActiveProcessLimit`) |
| **Syscall Filtering** | **Configurable** (Seccomp-BPF) | **None** (No native unprivileged filter) | **None** |
| **Capability Level** | **`CapabilityTier::FullConfinement`** | **`CapabilityTier::GuardedProcess`** | **`CapabilityTier::GuardedProcess`** |

---

## 4. Strict vs. Guarded Enforcement Contracts

```
                            [ SECE Run Request ]
                                     │
                                     ▼
                      [ Inspect Platform Capabilities ]
                                     │
                 ┌───────────────────┴───────────────────┐
                 ▼                                       ▼
     [ EnforcementMode::Strict ]             [ EnforcementMode::Guarded ]
                 │                                       │
      Is CapabilityTier == Full?              Is CapabilityTier >= Guarded?
        ┌────────┴────────┐                     ┌────────┴────────┐
       Yes                No                   Yes                No
        │                  │                    │                  │
        ▼                  ▼                    ▼                  ▼
 [ Run Isolated ]   [ REFUSE EXECUTION ] [ Warn & Run ]   [ REFUSE EXECUTION ]
                    (Fail-Closed Error)   (Audit Trail)    (Fail-Closed Error)
```

### 4.1 `EnforcementMode::Strict` (Mandatory for Untrusted Code)
- Requires **100% full confinement** across all 5 invariants (Filesystem + Network + Process Tree + Memory + Syscalls).
- Supported natively on **Linux** with unprivileged user namespaces.
- On macOS and Windows, **execution is rejected immediately**:
  ```
  ❌ Sandbox Security Error: Host platform cannot enforce strict isolation.
     Missing capabilities: [FilesystemNamespace, NetworkNamespace, ProcessTreeImmunity].
     Refusing to execute unverified candidate code on host.
  ```

### 4.2 `EnforcementMode::Guarded` (Developer Local Experimentation)
- Permits execution on Tier 2 platforms (macOS / Windows) when the developer explicitly opts into guarded development.
- Enforces all achievable native limits:
  - macOS: Process group kill (`SIGKILL` to `-pgid`), wall-clock timeouts, stream truncation, ephemeral directory purging.
  - Windows: Job Object limits (`KILL_ON_JOB_CLOSE`, memory limit, process count limit).
- **Mandatory Audit Logging:** Emits explicit warnings that network and filesystem isolation are inactive.

---

## 5. Rust Crate API Design (`crates/sandbox`)

The crate architecture is structured around clean interfaces, zero code duplication, and strict capability checking.

### 5.1 Crate Module Hierarchy

```
crates/sandbox/src/
├── lib.rs                  # Public façade, SECE engine, and backward-compatible APIs
├── traits.rs               # SandboxProvider and IsolatedChild traits
├── capabilities.rs         # Platform capability structs, enums, and detection
├── config.rs               # SandboxConfig, ResourceLimits, EnforcementMode
├── error.rs                # Typed SandboxError and RefusalReason
├── supervisor.rs           # Process lifecycle supervisor and output collector
└── platform/               # OS-specific providers
    ├── mod.rs              # Dynamic provider selection
    ├── linux.rs            # Linux namespaces, cgroups, seccomp
    ├── macos.rs            # macOS posix_spawn, process groups, resource limits
    ├── windows.rs          # Windows Job Objects, process limits
    └── fallback.rs         # Unsupported platform fail-closed provider
```

---

### 5.2 Core Types & Trait Definitions

#### `capabilities.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityTier {
    /// Full kernel-enforced confinement (Filesystem + Network + PID + Quotas)
    FullConfinement,
    /// Partial host process guard (Timeouts, Process Groups, Job Objects)
    GuardedProcess,
    /// Unconfined execution (No isolation primitives available)
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub tier: CapabilityTier,
    pub filesystem_isolation: bool,
    pub network_isolation: bool,
    pub process_tree_containment: bool,
    pub memory_quota_enforced: bool,
    pub cpu_time_enforced: bool,
    pub syscall_filtering: bool,
    pub platform_name: &'static str,
}
```

#### `config.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    /// Refuse execution if full confinement is unavailable
    Strict,
    /// Allow guarded execution with explicit warnings on Tier 2 platforms
    Guarded,
    /// Opt-in raw execution (internal unit testing only)
    UnsafeDisabled,
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub wall_clock_timeout: Duration,
    pub cpu_time_limit: Option<Duration>,
    pub memory_limit_bytes: Option<u64>,
    pub max_pids: Option<u32>,
    pub max_output_bytes: usize,
    pub max_disk_write_bytes: Option<u64>,
}
```

#### `error.rs`
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    /// Refusal to execute because platform cannot provide required guarantees
    CapabilityRefusal {
        missing: Vec<&'static str>,
        tier: CapabilityTier,
        mode: EnforcementMode,
    },
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

#### `traits.rs`
```rust
/// Unified security contract implemented by each operating system backend
pub trait SandboxProvider: Send + Sync {
    /// Queries the verified security capabilities of this host platform
    fn capabilities(&self) -> PlatformCapabilities;

    /// Spawns an isolated candidate process enforcing the configured limits
    fn spawn(
        &self,
        binary_path: &Path,
        args: &[String],
        config: &SandboxConfig,
    ) -> Result<Box<dyn IsolatedChild>, SandboxError>;
}

/// Handle to a running sandboxed candidate
pub trait IsolatedChild {
    /// Polls execution status without blocking
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, SandboxError>;

    /// Blocks until process exits or timeout occurs, enforcing resource limits
    fn wait_timeout(&mut self, timeout: Duration) -> Result<RunOutput, SandboxError>;

    /// Terminates the entire process tree immediately with SIGKILL
    fn kill_tree(&mut self) -> Result<(), SandboxError>;
}
```

---

## 6. Backward Compatibility Strategy

To ensure zero regression across existing Syntra tools (`syn experiment`, `syn tune`, `syn approve`, and `syn build`), the existing public interfaces in `crates/sandbox/src/lib.rs` and `runner.rs` are preserved:

1. **`SandboxRunner`:** Retains existing public methods (`new`, `with_timeout`, `with_max_output_bytes`, `compile_candidate`, `execute_binary`, `cleanup`). Internally delegates to `platform::create_default_provider()`.
2. **`EquivalenceVerifier`:** Unchanged signature. Compares baseline and candidate `RunOutput`.
3. **`BenchmarkEvaluator`:** Unchanged signature. Evaluates statistical Pareto metrics.
4. **`ExperimentalCompilationEngine`:** Retains `run_experiment()` and `approve_candidate()`. Respects toolchain enforcement mode.

---

## 7. Adversarial Test Integration Plan

Deliverable 03 defines the exact verification harness to be implemented alongside the platform providers:

```rust
#[test]
fn test_strict_mode_rejects_insufficient_platform() {
    let mock_provider = MockProvider::new(CapabilityTier::GuardedProcess);
    let config = SandboxConfig::new(EnforcementMode::Strict);
    let res = mock_provider.spawn(Path::new("/bin/true"), &[], &config);
    assert!(matches!(res, Err(SandboxError::CapabilityRefusal { .. })));
}
```

Tests to include:
- `test_double_fork_tree_kill`: Validates that descendant processes are killed.
- `test_memory_ceiling_kill`: Validates that memory limits terminate runaway allocations.
- `test_network_connectivity_blocked`: Validates that outbound network calls fail.
- `test_output_truncation`: Validates stream bounds.
- `test_fail_closed_on_capability_mismatch`: Validates strict rejection.

---

## 8. Conclusion & Sign-Off Checklist

Deliverable 03 establishes:
1. **Accurate capability classifications** without false claims.
2. **Fail-Closed rejection** for platforms lacking required guarantees.
3. **A unified Rust trait architecture** for platform implementations.
4. **100% backward compatibility** with the Syntra compiler pipeline.

*Deliverable 03 is complete. No implementation code will be written until this specification is reviewed.*
