# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 07A: Independent Architectural Red-Team Review

**Audited Components:** Stage 6 Architectural Deliverables (`06A`, `06B`, `06C`, `06D`)  
**Target Milestone:** Phase 2, Stage 6.5 — Independent Architectural Red-Team Review  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **RED-TEAM AUDIT COMPLETE — CRITICAL DEFECTS IDENTIFIED**

---

## 1. Executive Summary & Objective

In accordance with Stage 6.5 governance directives, this document performs an unsparing, adversarial red-team review of the Stage 6 implementation architecture.

While Stage 6 substantially improved upon previous drafts by eliminating naive boolean flags and proposing decoupled modules, **adversarial inspection reveals four critical architectural defects and three subtle kernel race conditions** that would undermine sandbox containment in production if implemented as written.

### Headline Red-Team Findings:
1. **The Process-Wide File Descriptor Fallacy (CRITICAL DEFECT):** Deliverable 06A attempted to resolve the `getppid() != supervisor_tid` bug by introducing a `sync_pipe`. However, file descriptors in POSIX are **process-wide, not thread-owned**. If a worker supervisor thread terminates or panics, its file descriptors remain open in the parent process table. The child process will never observe `EOF` on the pipe, invalidating supervisor-death detection.
2. **Premature Candidate Execution Window (CRITICAL DEFECT):** In Deliverable 06A/06B, candidate process creation and cgroup attachment lacked an explicit sequential state machine. If candidate instructions execute before cgroup controller attachment, a malicious candidate can allocate host memory before limits apply.
3. **Windows Job Breakaway Race (CRITICAL DEFECT):** On Windows, creating a process without `CREATE_SUSPENDED` introduces a race where candidate code can spawn child processes or consume memory before `AssignProcessToJobObject` completes.
4. **Coarse Capability Conflation (ARCHITECTURAL DEFECT):** Deliverable 06B conflated operating system feature discovery with runtime enforcement and independent verification. Probing that Linux supports namespaces does not establish that a given candidate execution was successfully isolated.

---

## 2. Mandatory Review Area A: Supervisor Lifecycle & File Descriptor Ownership

### 2.1 The Process-Wide File Descriptor Flaw
*Affected Deliverable:* Deliverable 06A, Section 2.2 ("Parent Thread ID vs. Process ID in PR_SET_PDEATHSIG").

*The Proposed Claim in 06A:*
> "parent-child survival is synchronized via a dedicated unidirectional POSIX pipe (`sync_pipe`) created with `O_CLOEXEC`. If the supervisor thread dies for any reason, the kernel automatically closes the parent's pipe write end, delivering an instant `EOF` / `EPIPE` to the child, which immediately self-terminates before `execve()`."

*Kernel Reality & Exploitation Scenario:*
1. Under POSIX.1 and Linux kernel semantics (`fs/file.c`), file descriptors are entries in `task_struct->files->fdt`.
2. In a multi-threaded Rust process (e.g. the Syntra compiler running parallel compilation or SECE optimization loops), all threads share the **same file descriptor table** (`CLONE_FILES`).
3. If the supervisor is executed on a worker thread that panics or terminates while the main compiler process survives:
   - The kernel does **not** close the file descriptor held by the terminated thread.
   - The write end of `sync_pipe` remains active in the process file table.
   - The child process reading from `sync_pipe` does **not** receive `EOF`. It remains blocked indefinitely in `read()`, or if it already proceeded, it never receives an asynchronous termination notification.
4. Furthermore, `prctl(PR_SET_PDEATHSIG, SIGKILL)` is tied to the thread group leader if the process was created via standard `fork()`, or to the calling thread if created via `clone()`. If a worker thread finishes its work normally and exits, the child is unexpectedly killed by `SIGKILL` despite the parent compiler remaining healthy.

*Red-Team Conclusion:* The in-process supervisor thread model is fundamentally flawed when relying on file descriptor lifecycle. Supervisor ownership must either be encapsulated in a **dedicated supervisor process (Two-Tier Architecture)** or guarded by explicit process-level RAII lifecycle barriers.

---

## 3. Mandatory Review Area B: Linux Process Isolation & Spawning Sequence

### 3.1 Cgroup Attachment Ordering & The Memory Race Window
*Affected Deliverable:* Deliverable 06A, Section 2.4 & Deliverable 06B, Section 5.1.

*The Flaw:*
Stage 6 specified that the child is spawned into namespaces and then attached to Cgroups v2. However, the exact handshake sequence failed to guarantee that **zero instructions of the candidate binary execute prior to cgroup attachment**.

*Kernel Exploitation Scenario:*
1. The intermediary process forks the candidate process.
2. The candidate process begins dynamic linker initialization (`/lib64/ld-linux-x86-64.so.2`).
3. Before the parent supervisor writes the child's host PID to `/sys/fs/cgroup/syntra_sandbox/<run_id>/cgroup.procs`, the candidate executes code.
4. A malicious candidate rapidly calls `mmap()` or `sbrk()` to allocate 2 GB of memory.
5. Because the candidate is not yet in the cgroup, this memory is charged to the **compiler parent process cgroup**, completely evading `memory.max` and potentially triggering host OOM killing of the parent compiler!

*Red-Team Correction:* The candidate process **must be completely halted prior to `execve()`**. The child must enter a synchronous barrier, wait for the parent to write its host PID to `cgroup.procs`, verify that `/proc/self/cgroup` reflects the containment cgroup, and only then proceed.

### 3.2 Namespace Trampoline PID 1 Semantics
*Affected Deliverable:* Deliverable 06A, Section 2.3.

*Kernel Reality:*
When `unshare(CLONE_NEWPID)` is called, the calling process is **not** moved into the new PID namespace. Only children created via subsequent `fork()` calls enter the new namespace.
If the intermediary process executes the candidate directly, the candidate is **not** PID 1.
To make the candidate PID 1 in the new namespace, the sequence must be:
$$\text{Supervisor (Host)} \xrightarrow{\text{fork}} \text{Intermediary} \xrightarrow{\text{unshare(CLONE\_NEWPID)}} \text{Intermediary} \xrightarrow{\text{fork}} \text{Candidate (PID 1)}$$
The intermediary process must remain alive to act as the namespace reaper, or it must cleanly exit and pass ownership to a dedicated supervisor.

---

## 4. Mandatory Review Area C: Windows Job Object Assignment

### 4.1 The Process Breakaway Race Window
*Affected Deliverable:* Deliverable 06A, Section 2.6 & Deliverable 06B, Section 5.3.

*The Flaw:*
Stage 6 noted that breakaway rights should be disallowed. However, it failed to mandate that **the process must be created in a suspended state**.

*Windows Kernel Reality & Exploitation Scenario:*
1. If `CreateProcessW` is invoked without `CREATE_SUSPENDED`, the Windows kernel immediately creates the initial thread and schedules it on an available CPU core.
2. Before the supervisor thread calls `AssignProcessToJobObject(hJob, pi.hProcess)`:
   - The candidate executable runs its C runtime initialization (`mainCRTStartup`).
   - The candidate calls `CreateProcessW` to spawn a grandchild daemon.
   - Because the parent candidate is not yet assigned to the Job Object, the grandchild is created outside any Job Object!
   - When the supervisor finally assigns the parent process to the Job Object, the grandchild has already escaped containment.
3. Furthermore, if the candidate allocates 1 GB of committed memory before `AssignProcessToJobObject` completes, it bypasses `JOB_OBJECT_LIMIT_JOB_MEMORY`.

*Red-Team Correction:* Spawning on Windows must strictly require:
1. `CreateProcessW(..., CREATE_SUSPENDED, ...)`.
2. `AssignProcessToJobObject(hJob, pi.hProcess)`.
3. `SetInformationJobObject(hJob, JobObjectExtendedLimitInformation, ...)`.
4. `ResumeThread(pi.hThread)`.

---

## 5. Mandatory Review Area D: macOS Security Boundary Realities

### 5.1 The Fallacy of macOS "Guarded" Process Confinement
*Affected Deliverable:* Deliverable 06B, Section 2 & Section 5.2.

*The Flaw:*
Stage 6 described macOS as supporting `EnforcementMode::Guarded` with "process-group termination quotas".

*Darwin Kernel Reality:*
1. On macOS Darwin (XNU), calling `setpgid(0, 0)` creates a new process group.
2. A candidate process can immediately call `setsid()` to become a session leader and discard the process group.
3. If it calls `fork()`, the descendant belongs to a new process group.
4. Calling `libc::kill(-pgid, SIGKILL)` from the supervisor **fails to terminate any process that called `setsid()`**.
5. Unlike Linux (`CLONE_NEWPID`) or Windows (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), macOS POSIX process groups provide **zero immune containment**.

*Red-Team Mandate:* Deliverable 06B correctly marks `Strict` mode as `Unavailable` on macOS. However, the documentation for `Guarded` mode must explicitly warn developers that **macOS provides zero process-tree containment immunity against adversarial evasion**. Guarded mode on macOS is purely cooperative.

---

## 6. Mandatory Review Area E: The 5-Level Capability Verification Model

### 6.1 The Feature Discovery vs. Enforcement Gap
*Affected Deliverable:* Deliverable 06A, Section 4 & Deliverable 06B, Section 3.

*The Flaw:*
Stage 6 introduced `CapabilityState` (`Enforced`, `Partial`, `Unavailable`, `Unverified`), but assigned these states based on **host environment feature discovery**.
For example:
- "Probe if `/proc/sys/kernel/unprivileged_userns_clone == 1` -> set `filesystem_mount_confinement = Enforced`."

*The Security Vulnerability:*
- The fact that the host kernel supports user namespaces does **not** mean that this particular candidate process was successfully isolated!
- An error could occur during `mount()`, a cgroup write could fail due to disk quota, or an inherited file descriptor could leak past `close_range`.
- Conflating **host capability discovery** with **execution-time enforcement verification** is a critical security vulnerability.

*The 5-Level Capability Model:*
The capability architecture must track the lifecycle of every isolation guarantee across five formal phases:
1. **Discovered:** Host OS provides the kernel primitive.
2. **Authorized:** Current process has credentials/permissions to invoke it.
3. **Configured:** Parameters (memory limits, namespaces, mount flags) are applied.
4. **Enforced:** Kernel confirms primitive activation (cgroup attachment verified, seccomp active).
5. **Verified:** Independent probe inside the isolated execution context confirms the security barrier holds.

---

## 7. Mandatory Review Area F: Public API Compatibility Verification

### 7.1 Empirical Inspection of Consumers
*Affected Deliverable:* Deliverable 06A, Section 7 & Deliverable 06C, Section 4.

We independently inspected all consumers across the repository:
1. `crates/cli/src/lib.rs` (lines 400–480):
   - Invokes `syntra_sandbox::ExperimentalCompilationEngine::run_experiment(...)`.
   - Invokes `syntra_sandbox::ExperimentalCompilationEngine::approve_candidate(...)`.
   - Invokes `syntra_sandbox::ApprovedStrategy::load_from_dir(...)`.
2. `crates/sandbox/src/lib.rs` (lines 170–290):
   - Constructs `SandboxRunner::new(run_id.clone())`.
   - Calls `runner.compile_candidate(...)`.
   - Calls `runner.execute_binary(...)`.
3. `crates/sandbox/tests/test_sandbox_isolation.rs` (lines 20–120):
   - Constructs `SandboxRunner::new(run_id).with_timeout(...)`.
   - Calls `runner.candidate_dir(...)`.
   - Calls `runner.execute_binary(...)`.

*Finding:*
The proposed `SandboxRunner` facade preserves every public method, constructor, and accessor without signature alteration. The backward compatibility contract is **sound**, provided that `RunOutput` maintains its exact field names (`stdout`, `stderr`, `exit_code`, `duration`, `timed_out`).

---

## 8. Summary of Red-Team Review Findings

| ID | Severity | Area | Defect Summary | Required Red-Team Correction |
| :- | :---: | :--- | :--- | :--- |
| **RED-01** | **CRITICAL** | Supervisor Lifecycle | Process-wide FD sharing breaks thread-death detection via pipe. | Implement Two-Tier Process Architecture or Process-Wide Signal Monitors. |
| **RED-02** | **CRITICAL** | Linux Cgroups | Candidate runs unbounded code before cgroup attachment. | Synchronous pre-exec barrier; attach PID and verify before releasing child. |
| **RED-03** | **CRITICAL** | Windows Job | Process created active before Job Object assignment. | Mandatory `CREATE_SUSPENDED` + Job assignment + `ResumeThread`. |
| **RED-04** | **MAJOR** | Capability Model | Conflated host discovery with execution-time verification. | Adopt 5-Level Capability Lifecycle Model (`Discovered` to `Verified`). |
| **RED-05** | **MAJOR** | Linux Namespaces | `unshare(CLONE_NEWPID)` doesn't make caller PID 1. | Formalize intermediary trampoline double-fork architecture. |
| **RED-06** | **MODERATE** | macOS Guarded | Process groups bypassable via `setsid()`. | Document zero process-tree immunity; strictly enforce fail-closed on Strict. |

*Stage 6.5 Red-Team Review Complete. Proceeding to Deliverable 07B (Critical Corrections Addendum).*
