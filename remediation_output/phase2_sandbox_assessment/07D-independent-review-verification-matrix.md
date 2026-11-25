# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 07D: Independent Review Verification Matrix

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 — Independent Review Verification Matrix  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **AUTHORITATIVE VERIFICATION MATRIX — READY FOR AUDIT REVIEW**

---

## 1. Executive Summary & Purpose

This document provides the formal **Traceability & Verification Matrix** for Stage 6.5. For every finding uncovered during the architectural red-team review, this matrix establishes:
1. Affected deliverable and section.
2. The flawed assumption or missing guarantee.
3. The underlying operating system / kernel behaviour.
4. The practical exploitation or failure scenario.
5. The corrected architectural contract.
6. The exact tests required to verify the correction.

---

## 2. Red-Team Findings & Verification Matrix

| Finding ID & Severity | Affected Deliverable | Flawed Assumption | OS / Kernel Reality | Exploitation / Failure Scenario | Corrected Contract | Required Test Verification |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **RED-01**<br>*(CRITICAL)* | Deliverable 06A (§2.2), Deliverable 06B (§5.1) | Pipe closure reliably detects supervisor worker thread death. | In POSIX, file descriptors are **process-wide** (`CLONE_FILES`), not thread-owned. Thread termination does not close FDs. | If a worker supervisor thread panics/terminates, write end remains open in parent compiler; child never receives EOF and blocks forever. | **Two-Tier Supervisor Process:** Dedicated helper process owns independent FD table. Process termination atomically closes FDs. | `test_adversarial_supervisor_process_death_signals_child` |
| **RED-02**<br>*(CRITICAL)* | Deliverable 06A (§2.4), Deliverable 06B (§5.1) | Candidate can be spawned and then attached to cgroups v2 asynchronously. | If candidate child begins executing before parent writes PID to `cgroup.procs`, it runs unconstrained during that window. | Malicious candidate allocates 2 GB memory before cgroup attachment; memory is charged to host parent compiler, evading `memory.max`. | **Synchronous Pre-Exec Barrier (7-Step Machine):** Candidate blocks on `sync_pipe` read before `execve()`; parent attaches and verifies cgroup before releasing child. | `test_adversarial_cgroup_pre_exec_memory_barrier` |
| **RED-03**<br>*(CRITICAL)* | Deliverable 06A (§2.6), Deliverable 06B (§5.3) | Windows Job Object limits can be applied to an actively running child process. | In Windows, active processes immediately schedule threads. The child can fork or consume resources before Job assignment. | Candidate calls `CreateProcessW` before `AssignProcessToJobObject` completes, allowing grandchild to escape Job Object. | **Mandatory `CREATE_SUSPENDED`:** Spawns suspended, assigns Job Object, sets limits, and only then calls `ResumeThread`. | `test_adversarial_windows_job_suspended_spawning` |
| **RED-04**<br>*(MAJOR)* | Deliverable 06A (§4), Deliverable 06B (§3) | System feature discovery (e.g. `unprivileged_userns_clone == 1`) implies execution-time enforcement. | OS feature support does not prove a specific sandbox container successfully applied mount/network boundaries without errors. | Mount fails or permissions dropped silently; supervisor reports "Enforced" despite candidate running on host filesystem. | **5-Level Capability Lifecycle Model:** Distinguishes `Discovered`, `Authorized`, `Configured`, `Enforced`, and `Verified` via in-container probes. | `test_adversarial_dynamic_capability_runtime_probes` |
| **RED-05**<br>*(MAJOR)* | Deliverable 06A (§2.3), Deliverable 06B (§5.1) | `unshare(CLONE_NEWPID)` makes the calling process PID 1 in the new namespace. | `unshare(CLONE_NEWPID)` only affects **subsequent children** created via `fork()`. Calling process remains in parent namespace. | Direct execution leaves candidate in parent PID namespace or fails with invalid init process state. | **Trampoline Double-Fork Architecture:** Intermediary process unshares PID namespace and forks inner child; inner child becomes true PID 1. | `test_adversarial_double_fork_daemon` (ADV-01) |
| **RED-06**<br>*(MODERATE)*| Deliverable 06B (§2, §5.2) | macOS process groups provide "Guarded process-tree containment". | On Darwin, any process can call `setsid()` to become a session leader and escape process-group `kill(-pgid)`. | Candidate calls `setsid()` and forks a daemon; `kill(-pgid)` fails to kill daemon; daemon survives sandbox run. | **Honest Boundary Contract:** Explicitly document zero process-tree immunity on macOS; strictly fail closed in Strict mode. | `test_adversarial_strict_mode_fail_closed` (ADV-09) |

---

## 3. Operational Risks & Technical Assumptions Matrix

| Risk ID & Category | Operational Risk / Assumption | Underlying Mechanism | Potential Impact | Mitigating Architectural Contract |
| :--- | :--- | :--- | :--- | :--- |
| **RISK-01**<br>*(Container CI)* | Rootless Cgroups v2 unavailable in GitHub Actions / Docker. | Unprivileged containers may lack write access to `/sys/fs/cgroup/cgroup.subtree_control`. | Sandbox cannot create nested cgroup for `memory.max`. | Dynamic probe marks `memory_allocation_ceiling = Partial`. Strict mode fails closed with explicit remediation guide. |
| **RISK-02**<br>*(Dynamic Linker)* | Cranelift-linked candidate fails with `ENOENT` in minimal mount namespace. | Candidate binary requires `/lib64/ld-linux-x86-64.so.2` and libc. | Candidate immediately crashes on `execve()` in isolated tmpfs. | Linux provider bind-mounts `/lib`, `/lib64`, `/usr/lib` into container tmpfs as read-only (`MS_RDONLY \| MS_BIND \| MS_REC`). |
| **RISK-03**<br>*(Darwin Signals)* | Detached process group on macOS receives `SIGTTOU` on terminal access. | Background process group attempting terminal writes suspended by kernel. | Candidate process hangs indefinitely waiting for tty. | macOS provider redirects `stdin` to `/dev/null` and ignores `SIGTTOU`/`SIGTTIN` in child pre-exec. |
| **RISK-04**<br>*(Stream Deadlock)* | Child floods pipe with 100 MB while supervisor drains. | OS pipe buffers (default 64 KB on Linux) fill up; child blocks on `write()`. | Supervisor deadlocks if draining synchronously. | Non-blocking `poll()` loop with independent buffers; terminates process immediately on overflow. |

---

## 4. Adversarial Verification Test Mapping (ADV-01 through ADV-10)

Every red-team finding maps directly to the Phase 2 adversarial test suite (`crates/sandbox/tests/test_adversarial_isolation.rs`):

```
┌────────────────────────────────────────────────────────────────────────┐
│               ADVERSARIAL SUITE TRACEABILITY MAPPING                   │
├────────────────────────────────────────────────────────────────────────┤
│  ADV-01 (Double-fork daemon)        ◄──► Verifies RED-01, RED-05       │
│  ADV-02 (Credential access)         ◄──► Verifies RED-04               │
│  ADV-03 (Network egress probe)      ◄──► Verifies RED-04               │
│  ADV-04 (Memory allocation bomb)    ◄──► Verifies RED-02               │
│  ADV-05 (Fork bomb proliferation)   ◄──► Verifies RED-01, RED-05       │
│  ADV-06 (Inherited FD access)       ◄──► Verifies RED-01               │
│  ADV-07 (io_uring evasion)          ◄──► Verifies Seccomp filter       │
│  ADV-08 (Stream flooding & drain)   ◄──► Verifies RISK-04              │
│  ADV-09 (Fail-closed strict mode)   ◄──► Verifies RED-06               │
│  ADV-10 (Procfs write tampering)    ◄──► Verifies Mount isolation      │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Summary of Deliverable 07D

This matrix provides complete traceability:
- All 6 red-team findings (RED-01 to RED-06) are linked to kernel causes, exploitation vectors, and concrete tests.
- All 4 operational risks (RISK-01 to RISK-04) have defined architectural mitigations.
- Zero open assumptions remain unmapped prior to Stage 7 implementation.

*Deliverable 07D complete. Stage 6.5 Deliverables Complete.*
