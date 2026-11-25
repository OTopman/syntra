# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 07B: Critical Corrections & Security Contract Addendum

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 — Critical Corrections & Security Contract Addendum  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **AUTHORITATIVE CONTRACT ADDENDUM — SUPERSEDES STAGE 6 DEFECTS**

---

## 1. Executive Summary & Purpose

This document provides the formal **Critical Corrections & Security Contract Addendum** resolving the architectural defects identified in Deliverable 07A.

This addendum supersedes conflicting specifications in Deliverables 06A, 06B, 06C, and 06D. It establishes:
1. **The Two-Tier Process Supervisor Architecture**, resolving the process-wide file descriptor lifecycle flaw.
2. **The 7-Step Linux Process Isolation State Machine**, closing the premature candidate execution window.
3. **The Windows `CREATE_SUSPENDED` Job Object Assignment Contract**, eliminating process breakaway races.
4. **The 5-Level Capability Lifecycle Model**, distinguishing discovery, authorization, configuration, enforcement, and verification.
5. **The Hardened macOS Security Contract**, establishing honest, non-immune boundaries in Guarded mode.

---

## 2. Correction 1: Two-Tier Process Supervisor Architecture

### 2.1 The Architectural Defect (RED-01)
In Deliverable 06A, supervision was assigned to a persistent worker thread within the multi-threaded compiler process. Because file descriptors in POSIX are process-wide (`task_struct->files`), thread termination or panic does **not** close file descriptors. Consequently, synchronization pipes cannot reliably signal thread death.

### 2.2 The Two-Tier Architecture Specification

```
┌────────────────────────────────────────────────────────────────────────┐
│                   TWO-TIER SUPERVISOR ARCHITECTURE                     │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  [ Tier 0: Compiler Host Process (syntra / syn) ]                     │
│    • Multi-threaded compiler runtime (Rayon / Tokio / Cranelift)      │
│    • Coordinates optimization passes and SECE candidate generation     │
│                     │                                                  │
│                     ▼ spawns dedicated helper process                  │
│  [ Tier 1: Dedicated Sandbox Supervisor Process ]                      │
│    • Single-threaded, dedicated process (PID S)                        │
│    • Owns independent file descriptor table and signal mask           │
│    • Bound to Tier 0 via control pipe (O_CLOEXEC)                      │
│    • If Tier 0 dies, Tier 1 receives SIGKILL instantly                │
│    • Acts as PR_SET_CHILD_SUBREAPER for candidate tree                 │
│                     │                                                  │
│                     ▼ forks and unshares namespaces                    │
│  [ Tier 2: Candidate Execution Container ]                             │
│    • Runs in unshared PID, User, Mount, and Network namespaces        │
│    • PR_SET_PDEATHSIG bound to Tier 1 process PID S                    │
│    • If Tier 1 process terminates, kernel closes its FDs and sends     │
│      SIGKILL to Tier 2 container PID 1                                 │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

#### Invariants of the Two-Tier Architecture:
1. **True Process-Level FD Ownership:**
   - The Tier 1 supervisor is a distinct process, spawned via `/proc/self/exe --sandbox-supervisor <run_id>` (or a specialized helper binary).
   - Because Tier 1 is a separate process, when it terminates for any reason (crash, OOM, or normal exit), the OS kernel's `exit_group` routine automatically and atomically closes all file descriptors held by Tier 1.
2. **Deterministic Pipe Closure:**
   - The candidate container in Tier 2 holds the read end of the synchronization pipe. The Tier 1 supervisor holds the write end.
   - If Tier 1 crashes or is killed, the kernel closes the write end immediately, delivering an unforgeable `EOF` to Tier 2.
3. **Unambiguous `PR_SET_PDEATHSIG`:**
   - In a single-threaded Tier 1 process, Thread ID (TID) == Process ID (PID) == Thread Group ID (TGID).
   - `prctl(PR_SET_PDEATHSIG, SIGKILL)` in the child is bound unambiguously to the supervisor process. When Tier 1 dies, the kernel delivers `SIGKILL` to Tier 2 without thread-versus-process confusion.

---

## 3. Correction 2: 7-Step Linux Process Isolation State Machine

### 3.1 The Architectural Defect (RED-02 & RED-05)
Candidate instructions must never execute before cgroup controller attachment and namespace isolation are established.

### 3.2 The Strict 7-Step State Machine

```
   State 0: [ Unshared Trampoline ]
            │  Tier 1 supervisor unshares User, Mount, Net, PID namespaces
            ▼
   State 1: [ Intermediary Fork ]
            │  Intermediary process forks; inner child becomes PID 1 in new PID namespace
            ▼
   State 2: [ Synchronous Candidate Pause ]
            │  Inner child blocks on read(sync_pipe, &token, 1);
            │  ZERO candidate instructions or dynamic linker code execute
            ▼
   State 3: [ Host Cgroup Attachment ]
            │  Tier 1 supervisor obtains host PID of inner child;
            │  Writes host PID to /sys/fs/cgroup/syntra_sandbox/<run_id>/cgroup.procs
            ▼
   State 4: [ Cgroup Configuration & Verification ]
            │  Tier 1 writes memory.max, memory.swap.max = 0, memory.oom.group = 1, pids.max;
            │  Reads back /proc/<host_pid>/cgroup to verify active containment
            ▼
   State 5: [ Container Security Policies ]
            │  Tier 1 writes go-ahead token (0x01) to sync_pipe;
            │  Child applies PR_SET_NO_NEW_PRIVS, installs Seccomp-BPF filter,
            │  detaches terminal (setsid), sanitizes descriptors (close_range)
            ▼
   State 6: [ Execution Release ]
            │  Inner child invokes execve(candidate_binary, args, env);
            ▼
   State 7: [ Supervision & Atomic Teardown ]
               Tier 1 monitors pidfd, non-blocking stream poll;
               Atomic kill via memory.oom.group or pidfd_send_signal
```

#### Detailed State Transition Invariants:
- **State 2 Invariant:** The candidate child is paused before `execve()`. No user code, no compiler runtime, and no external libraries have been loaded.
- **State 4 Invariant:** The host cgroup write must succeed and be verified before the release token is transmitted. If writing to `cgroup.procs` fails (e.g. permission or quota error), the child is immediately killed and execution aborts with `SandboxError::PlatformSetupFailure`.
- **State 5 Invariant:** Seccomp-BPF filters are installed with `PR_SET_NO_NEW_PRIVS = 1`, preventing privilege escalation across `execve`.

---

## 4. Correction 3: Windows `CREATE_SUSPENDED` Job Object Assignment

### 4.1 The Architectural Defect (RED-03)
Creating an active process on Windows before assigning it to a Job Object allows candidate instructions to run unconstrained and spawn breakaway processes.

### 4.2 The Suspended Spawning Contract

```rust
#[cfg(target_os = "windows")]
pub unsafe fn spawn_isolated_windows(
    binary_path: &Path,
    args: &[String],
    config: &SandboxConfig,
) -> Result<WindowsChild, SandboxError> {
    // 1. Create Job Object with strict breakaway restrictions
    let h_job = CreateJobObjectW(ptr::null_mut(), ptr::null());
    if h_job.is_null() {
        return Err(SandboxError::PlatformSetupFailure("CreateJobObjectW failed".into()));
    }

    // 2. Configure Limits before process creation
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
    limits.BasicLimitInformation.LimitFlags = 
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE |
        JOB_OBJECT_LIMIT_JOB_MEMORY |
        JOB_OBJECT_LIMIT_ACTIVE_PROCESSES;
    
    // Explicitly prohibit breakaway
    limits.BasicLimitInformation.LimitFlags &= !JOB_OBJECT_LIMIT_BREAKAWAY_OK;
    limits.BasicLimitInformation.LimitFlags &= !JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;
    
    limits.JobMemoryLimit = config.memory_allocation_ceiling_bytes;
    limits.BasicLimitInformation.ActiveProcessLimit = config.max_pids;

    SetInformationJobObject(
        h_job,
        JobObjectExtendedLimitInformation,
        &limits as *const _ as *mut _,
        mem::size_of_val(&limits) as u32,
    );

    // 3. Spawn candidate process in SUSPENDED state
    let mut si: STARTUPINFOW = mem::zeroed();
    si.cb = mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = mem::zeroed();

    let success = CreateProcessW(
        wide_path.as_ptr(),
        wide_args.as_mut_ptr(),
        ptr::null_mut(),
        ptr::null_mut(),
        FALSE, // bInheritHandles = FALSE
        CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP,
        ptr::null_mut(),
        wide_dir.as_ptr(),
        &si,
        &pi,
    );

    if success == 0 {
        CloseHandle(h_job);
        return Err(SandboxError::PlatformSetupFailure("CreateProcessW failed".into()));
    }

    // 4. Assign to Job Object while threads are halted
    let assigned = AssignProcessToJobObject(h_job, pi.hProcess);
    if assigned == 0 {
        TerminateProcess(pi.hProcess, 1);
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
        CloseHandle(h_job);
        return Err(SandboxError::PlatformSetupFailure("AssignProcessToJobObject failed".into()));
    }

    // 5. Resume execution inside the Job Object
    ResumeThread(pi.hThread);
    CloseHandle(pi.hThread);

    Ok(WindowsChild { h_job, h_process: pi.hProcess, pid: pi.dwProcessId })
}
```

*Contract Invariant:* The candidate process is assigned to the Job Object while its primary thread is suspended. Not a single instruction executes outside the Job Object.

---

## 5. Correction 4: The 5-Level Capability Lifecycle Model

### 5.1 The Architectural Defect (RED-04)
Conflating static operating system feature detection with dynamic execution enforcement.

### 5.2 The 5-Level Capability Architecture
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityLevel {
    /// 1. Discovered: Host OS supports the isolation primitive
    Discovered,
    /// 2. Authorized: Current user / process has credentials to use the primitive
    Authorized,
    /// 3. Configured: Isolation parameters have been staged for execution
    Configured,
    /// 4. Enforced: Kernel subsystem actively enforces containment
    Enforced,
    /// 5. Verified: In-container empirical probe confirms the barrier holds
    Verified,
}
```

### 5.3 Concrete Runtime Verification Probes
Before declaring an invariant verified, the supervisor runs an active probe:

| Invariant | Configuration Step | Verification Probe inside Container |
| :--- | :--- | :--- |
| **`filesystem_mount_confinement`** | Mount minimal tmpfs root | Attempt `stat("/etc/shadow")` or host user root. Must return `ENOENT`. |
| **`network_egress_blocking`** | `CLONE_NEWNET` + loopback DOWN | Query netlink `RTM_GETROUTE`. Must return 0 routes. Socket connect fails with `ENETUNREACH` or `ENETDOWN`. |
| **`memory_allocation_ceiling`** | Set `memory.max` + `memory.swap.max = 0` | Inspect `/proc/self/cgroup` and `/sys/fs/cgroup/.../memory.max`. Confirm values match configured ceiling. |
| **`process_tree_immune_kill`** | `CLONE_NEWPID` | Inspect `/proc/self/status` or `getpid()`. Must be PID 1 in namespace. |
| **`descriptor_sanitization`** | `close_range(3, ~0U, 0)` | Iterate `/proc/self/fd`. Must contain only FDs 0, 1, 2. |

---

## 6. Correction 5: Hardened macOS Security Boundaries

### 6.1 The Architectural Clarification (RED-06)
- **Strict Mode:** Confirmed strictly **Unavailable** on native macOS. Attempts to run untrusted candidate code on macOS in Strict mode will refuse execution without spawning any process.
- **Guarded Mode:** Explicitly documented as **non-immune**. Process groups (`setpgid`) and `kill(-pgid)` are cooperative and subject to evasion via `setsid()`.
- **Audit Requirement:** Whenever Guarded mode runs on macOS, the supervisor logs:
  ```text
  [SECE SECURITY WARNING: Cooperative Guarded Mode on macOS]
  Host platform lacks unprivileged filesystem, network, and process-tree containment.
  Process group kill cannot prevent setsid() evasion.
  Permitted ONLY for trusted developer optimization workloads.
  ```

---

## 7. Summary of Deliverable 07B

This addendum resolves all four critical defects:
1. Replaced the vulnerable in-process thread pipe with a **Two-Tier Supervisor Process** architecture.
2. Formulated the **Strict 7-Step Linux Spawning State Machine** with synchronous cgroup attachment before candidate execution.
3. Mandated **`CREATE_SUSPENDED`** on Windows before Job Object attachment.
4. Replaced coarse discovery flags with the **5-Level Capability Lifecycle Model**.
5. Explicitly defined macOS Guarded mode as cooperative and non-immune.

*Deliverable 07B complete. Proceeding to Deliverable 07C (Go/No-Go Readiness Assessment).*
