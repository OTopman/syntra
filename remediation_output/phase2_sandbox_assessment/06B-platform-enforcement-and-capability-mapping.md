# Syntra Phase 2: SECE Sandbox Security Architecture
# Deliverable 06B: Platform Enforcement & Capability Mapping Specification

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6 — Platform Enforcement & Capability Mapping  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **STAGE 6 SPECIFICATION — READY FOR REVIEW**

---

## 1. Executive Summary & Purpose

A core finding of Phase 2 (Deliverables 01 and 02) was that presenting a platform-agnostic facade without explicit, verified kernel capabilities constitutes a severe security defect. In the original `crates/sandbox` implementation, a timeout and cleared environment variables were falsely treated as "sandbox isolation" on macOS and Linux alike.

This document establishes the **authoritative Platform Enforcement & Capability Mapping** for Phase 2. It details:
1. The dynamic, evidence-backed capability detection protocol.
2. The concrete isolation primitives for Linux, macOS, Windows, and fallback platforms.
3. The strict vs. guarded execution contracts.
4. Explicit, non-negotiable failure modes when platform primitives are unavailable.

---

## 2. Platform Capability Matrix & 4-State Mapping

In accordance with Deliverable 03A, every security dimension is evaluated through a 4-state enum:
```rust
pub enum CapabilityState {
    Enforced,    // Guaranteed by native kernel mediation primitives
    Partial,     // Cooperative or subject to bypass without root privilege
    Unavailable, // Host kernel/OS cannot provide this isolation control
    Unverified,  // Capability not yet dynamically probed on this host instance
}
```

### Complete Cross-Platform Enforcement Matrix

| Isolation Invariant | Linux (Tier 1) | macOS Darwin (Tier 2) | Windows (Tier 2) | Fallback / Unknown | Mandatory in Strict? |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **`filesystem_mount_confinement`** | **Enforced** (`CLONE_NEWNS` + tmpfs pivot) | **Unavailable** (No user mnt ns) | **Unavailable** (Requires AppContainer) | **Unavailable** | **YES** |
| **`network_egress_blocking`** | **Enforced** (`CLONE_NEWNET` + route/link audit) | **Unavailable** (No user net ns) | **Unavailable** (No native user net filter) | **Unavailable** | **YES** |
| **`process_tree_immune_kill`** | **Enforced** (`CLONE_NEWPID` + `pidfd`) | **Partial** (`kill(-pgid)` broken by `setsid`) | **Enforced** (Job Object Close Kill) | **Unavailable** | **YES** |
| **`memory_allocation_ceiling`** | **Enforced** (Cgroups v2 `memory.max`) | **Unavailable** (`RLIMIT_AS` non-portable/soft) | **Enforced** (`JOB_OBJECT_LIMIT_JOB_MEMORY`) | **Unavailable** | **YES** |
| **`swap_elimination`** | **Enforced** (Cgroups v2 `memory.swap.max = 0`) | **Unavailable** (No swap control per process) | **Unavailable** (Paged pool shared) | **Unavailable** | **YES** |
| **`pid_table_ceiling`** | **Enforced** (Cgroups v2 `pids.max`) | **Partial** (`RLIMIT_NPROC` per-UID, not tree) | **Enforced** (`JOB_OBJECT_LIMIT_ACTIVE_PROCESSES`) | **Unavailable** | **YES** |
| **`standard_descriptor_sanitization`** | **Enforced** (`close_range` + `TIOCNOTTY`) | **Enforced** (`TIOCNOTTY` + `/proc/self/fd` sweep) | **Enforced** (`bInheritHandles = FALSE`) | **Unavailable** | **YES** |
| **`wall_clock_timeout`** | **Enforced** (Monotonic poll loop) | **Enforced** (Monotonic poll loop) | **Enforced** (Monotonic poll loop) | **Unavailable** | **YES** |
| **`seccomp_syscall_filtering`** | **Enforced** (Seccomp-BPF `io_uring` block) | **Unavailable** (No unprivileged seccomp) | **Unavailable** (No unprivileged syscall hook) | **Unavailable** | NO (Tier 1 Only) |

---

## 3. Dynamic Capability Detection Protocol

Capabilities must **never** be assumed solely from compile-time `#[cfg(target_os = "...")]` flags. Host environments vary: unprivileged user namespaces may be disabled by sysctl, Cgroups v2 might not be mounted in unified mode, or container environments may restrict seccomp.

The `capabilities.rs` module executes empirical runtime probes:

```rust
impl PlatformCapabilities {
    pub fn probe_host() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self::probe_linux_host()
        }
        #[cfg(target_os = "macos")]
        {
            Self::probe_macos_host()
        }
        #[cfg(target_os = "windows")]
        {
            Self::probe_windows_host()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Self::unsupported_host()
        }
    }
}
```

### 3.1 Linux Dynamic Probing Procedure
1. **User Namespaces Check:**
   - Probe `/proc/sys/kernel/unprivileged_userns_clone`. If present and set to `0`, user namespaces are disabled.
   - Empirical test: Attempt a trial `unshare(CLONE_NEWUSER)`. If it returns `EPERM`, mark `filesystem_mount_confinement` and `network_egress_blocking` as `Unavailable`.
2. **Cgroups v2 Unified Hierarchy Check:**
   - Verify `/sys/fs/cgroup/cgroup.controllers` exists.
   - Check if controllers `memory` and `pids` are enabled. If Cgroups v1 or hybrid hierarchy is present without delegation, mark `memory_allocation_ceiling` and `pid_table_ceiling` as `Partial`.
3. **Seccomp-BPF Support Check:**
   - Probe `prctl(PR_GET_SECCOMP)` or `prctl(PR_SET_NO_NEW_PRIVS, 1)`. If permitted, mark `seccomp_syscall_filtering` as `Enforced`.
4. **`close_range` Syscall Availability:**
   - Attempt `close_range(~0U, ~0U, 0)`. If returns `ENOSYS`, set fallback to `/proc/self/fd` iteration.

### 3.2 macOS Dynamic Probing Procedure
1. **Process Group & Signal Isolation:**
   - Mark `process_tree_immune_kill` as `Partial` (because candidates calling `setsid()` escape process-group signals).
2. **Filesystem & Network Isolation:**
   - Unconditionally marked as `Unavailable` (native XNU lacks containerless unprivileged namespaces; `sandbox-exec` is deprecated and inaccessible to user tools).
3. **Resource Limits:**
   - Test `setrlimit(RLIMIT_CPU)`. Memory limits (`RLIMIT_AS`) are marked `Unavailable` due to XNU's non-enforcing virtual memory semantics.

### 3.3 Windows Dynamic Probing Procedure
1. **Job Object Creation:**
   - Call `CreateJobObjectW(NULL, NULL)`. If successful, mark `process_tree_immune_kill` and `memory_allocation_ceiling` as `Enforced`.
2. **Filesystem & Network Virtualization:**
   - Mark as `Unavailable` (unprivileged AppContainer isolation requires package manifests and interactive desktop elevation outside the scope of compiler workers).

---

## 4. Strict vs. Guarded Enforcement Contracts

Syntra formally establishes two distinct execution contracts:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                    SECE ENFORCEMENT CONTRACT RULES                       │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  [ Untrusted Candidate / 3rd-Party Code ]                                │
│                     │                                                    │
│                     ▼ mandates                                           │
│          EnforcementMode::Strict                                         │
│                     │                                                    │
│       ┌─────────────┴─────────────┐                                      │
│       ▼                           ▼                                      │
│  [ Linux Host ]              [ macOS / Windows / Fallback ]              │
│  • Namespaces: OK            • Missing Filesystem Isolation: FAIL        │
│  • Cgroups v2: OK            • Missing Network Isolation: FAIL           │
│  • Evaluates Invariant       • Fails Closed:                             │
│    Conjunction: PASS           Err(StrictInvariantViolation)             │
│  • Executes Sandboxed        • ZERO PROCESSES SPAWNED                    │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  [ Trusted Local Developer Code ]                                        │
│                     │                                                    │
│                     ▼ allows explicit opt-in                             │
│          EnforcementMode::Guarded                                        │
│                     │                                                    │
│       ┌─────────────┴─────────────┐                                      │
│       ▼                           ▼                                      │
│  [ Linux Host ]              [ macOS / Windows ]                         │
│  • Runs with available       • Process-group kill & timeout only         │
│    protections               • Emits explicit Security Warning           │
│                              • Executes with monitored resources         │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Strict Mode Invariant Conjunction (Zero Trust)
In `Strict` mode, execution is evaluated against the conjunction of all non-negotiable invariants:
$$\text{StrictPermitted} = \bigwedge_{i \in \text{Mandatory}} (\text{Capability}[i] == \text{Enforced})$$

Where $\text{Mandatory}$ contains:
1. `filesystem_mount_confinement`
2. `network_egress_blocking`
3. `process_tree_immune_kill`
4. `memory_allocation_ceiling`
5. `wall_clock_timeout`
6. `standard_descriptor_sanitization`

If **any single mandatory invariant** is not `Enforced`, the platform provider refuses execution immediately:
```rust
if config.enforcement_mode == EnforcementMode::Strict {
    for (name, state) in self.capabilities().mandatory_invariants() {
        if state != CapabilityState::Enforced {
            return Err(SandboxError::StrictInvariantViolation {
                missing_invariant: name.to_string(),
                platform_state: format!("{:?}", state),
                remediation_hint: self.capabilities().remediation_hint(name),
            });
        }
    }
}
```

### 4.2 Guarded Mode Contract (Trusted Local Developer Workloads)
- `Guarded` mode relaxes filesystem and network requirements.
- It is **strictly prohibited for third-party packages, untrusted optimization candidates, or automated registry testing**.
- Permitted exclusively when a human developer runs local optimization tuning on their own source code (`syn experiment --guarded`).
- When activated on macOS or Windows, the toolchain prints an unmaskable security audit banner:
  ```text
  [WARNING: SECE Sandbox Running in Guarded Mode]
  Platform (macOS Darwin) does not provide kernel filesystem or network isolation.
  Candidate process is restricted by process-group termination quotas and timeouts only.
  Do not use Guarded mode on untrusted candidate code.
  ```

---

## 5. Platform Implementation Blueprints

### 5.1 Linux Implementation Blueprint (`platform/linux.rs`)
1. **User Namespace:** `unshare(CLONE_NEWUSER)` maps current UID/GID to container root (0/0) without setuid binaries.
2. **Mount Namespace:**
   - `mount(NULL, "/", NULL, MS_REC | MS_PRIVATE, NULL)` prevents mount leakage.
   - Mounts a minimal in-memory `tmpfs` as new root.
   - Mounts read-only container `/proc` private to the PID namespace.
   - Mounts isolated tmpfs on `/tmp`.
3. **Network Namespace:**
   - `unshare(CLONE_NEWNET)` creates isolated network stack.
   - Loopback interface left DOWN.
   - Verification probe confirms empty routing table and zero external interfaces.
4. **PID Namespace & Double-Fork:**
   - `unshare(CLONE_NEWPID)`.
   - Trampoline forks inner candidate; inner candidate becomes PID 1.
   - Intermediary supervisor reaps child and ensures namespace termination.
5. **Cgroups v2:**
   - Creates transient cgroup `/sys/fs/cgroup/syntra_sandbox/<run_id>`.
   - Writes `memory.max = <bytes>`, `memory.swap.max = 0`, `memory.oom.group = 1`, `pids.max = <max>`.
   - Attaches child host PID to `cgroup.procs` during synchronization handshake.
6. **Seccomp-BPF:**
   - Applies filter dropping `io_uring_setup`, `io_uring_enter`, `io_uring_register`, `ptrace`, `bpf`, `sys_chroot`, and raw packet sockets.

### 5.2 macOS Implementation Blueprint (`platform/macos.rs`)
1. **Process Confinement:**
   - In `pre_exec`, calls `setpgid(0, 0)` to establish a new process group.
   - Teardown issues `libc::kill(-pgid, libc::SIGKILL)` to terminate all processes in the group.
2. **Resource Quotas:**
   - Applies `setrlimit(RLIMIT_CPU, &limit)` for CPU quota.
   - Configures non-blocking stream readers on stdout/stderr.
3. **Strict Mode Failure:**
   - Unconditionally rejects `EnforcementMode::Strict` with `SandboxError::StrictInvariantViolation`.

### 5.3 Windows Implementation Blueprint (`platform/windows.rs`)
1. **Job Object Architecture:**
   - Creates a dedicated Job Object with `CreateJobObjectW`.
   - Sets `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`:
     - `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: Automatically kills all descendants when job handle drops.
     - `JOB_OBJECT_LIMIT_JOB_MEMORY`: Caps total aggregate committed memory across all descendant processes.
     - `JOB_OBJECT_LIMIT_ACTIVE_PROCESSES`: Caps fork bomb proliferation.
   - Disables `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` and breakaway rights.
2. **Process Spawning:**
   - Calls `CreateProcessW` with `CREATE_SUSPENDED`.
   - Calls `AssignProcessToJobObject(job_handle, process_handle)`.
   - Resumes process thread via `ResumeThread`.
3. **Strict Mode Failure:**
   - Unconditionally rejects `EnforcementMode::Strict` due to missing filesystem and network virtualization.

---

## 6. Summary of Deliverable 06B

Deliverable 06B establishes an honest, verified capability framework:
1. Replaces coarse aggregate labels with 4-state individual capability tracking.
2. Formulates the runtime dynamic detection protocol to verify kernel primitives before execution.
3. Enforces fail-closed Strict mode across macOS, Windows, and unverified Linux hosts.
4. Restricts Guarded mode to explicitly opted-in, trusted developer workloads.

*Deliverable 06B complete. Proceeding to Deliverable 06C.*
