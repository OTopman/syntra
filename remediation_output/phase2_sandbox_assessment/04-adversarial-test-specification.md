# Syntra Phase 2: SECE Sandbox Security Architecture
# Deliverable 04: Adversarial Test Specification & Escape Verification Matrix

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox  
**Target Milestone:** Phase 2 Adversarial Test Suite Definition  
**Author:** Principal Compiler Security Engineer & Systems Runtime Architect  
**Date:** 2026-11-24  
**Status:** **SPECIFICATION COMPLETE — READY FOR REVIEW**

---

## 1. Executive Summary & Purpose

In compiler supply-chain security, an isolation architecture cannot be certified through unit tests of happy paths alone. The existing `crates/sandbox` implementation passed 5 unit tests while remaining 100% unconfined on host filesystems and networks.

This document specifies the **formal, pre-implementation Adversarial Test Suite** for Phase 2. Every security invariant defined in Deliverables 02, 03, and 03B is mapped to an explicit adversarial attack scenario that must be executed and empirically verified before **FIND-002** can be closed.

---

## 2. Adversarial Test Harness Architecture

The test harness operates in `crates/sandbox/tests/test_adversarial_isolation.rs`.

Each test executes an adversarial payload (either an ELF/Mach-O native binary compiled from Syntra/Cranelift or a standalone adversarial mock binary), inspects the supervisor's return state, and probes the host environment post-execution to detect leaks.

```
┌───────────────────────────────────────────────────────────────────────────┐
│                       ADVERSARIAL TEST HARNESS                            │
│  Spawns Attack Payload via SandboxProvider -> Inspects SandboxError       │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │
          ┌───────────────────────────┼───────────────────────────┐
          ▼                           ▼                           ▼
[ Host Probing Post-Run ]   [ Process Tree Audit ]      [ Kernel Event Audit ]
  • Check if FDs leaked       • Verify no orphan PIDs     • Check Cgroups OOM
  • Check if files modified   • Verify session reaped     • Check Seccomp traps
```

---

## 3. Adversarial Attack Scenarios Matrix (ADV-01 through ADV-10)

### ADV-01: Double-Fork Daemon Detachment Escape
- **Threat Model:** A malicious or runaway candidate process spawns a child, which calls `setsid()` to become a session leader and forks a grandchild daemon before the intermediary child exits. The attacker expects the grandchild to be re-parented to host PID 1 (`init`/`launchd`) and evade `child.kill()`.
- **Attack Payload:**
  ```c
  if (fork() == 0) {
      setsid();
      if (fork() == 0) {
          while (1) { sleep(1); } // Detached daemon
      }
      exit(0);
  }
  wait(NULL);
  ```
- **Mandatory Invariant:**
  - Linux: PID namespace destruction terminates all descendants automatically.
  - macOS/Windows: Process group / Job Object termination.
- **Pass Verification:** Post-execution, the harness verifies via `/proc` or `kill(grandchild_pid, 0)` that the detached daemon is **completely dead** (`ESRCH`).

---

### ADV-02: Host Credential & Sensitive File Access
- **Threat Model:** The candidate attempts to read developer secrets from the host filesystem.
- **Attack Targets:**
  - `/etc/shadow`, `/etc/passwd`
  - `~/.ssh/id_rsa`, `~/.ssh/authorized_keys`
  - `~/.aws/credentials`, `~/.aws/config`
  - `.env` and `Package.lock` in the parent compiler workspace
- **Attack Payload:** Invocations of `open(path, O_RDONLY)`.
- **Pass Verification:**
  - In `Strict` mode, every attempt must fail with `ENOENT` (file does not exist in private mount namespace) or `EACCES`.
  - The test verifies that **zero bytes** of host secrets can be read.

---

### ADV-03: Outbound Network Egress & Local Port Probing
- **Threat Model:** The candidate attempts to exfiltrate host data or phone home to a Command & Control (C2) server.
- **Attack Targets:**
  - External DNS / UDP probe: `connect("8.8.8.8:53", SOCK_DGRAM)`.
  - External HTTP / TCP probe: `connect("1.1.1.1:80", SOCK_STREAM)`.
  - Localhost loopback probe: `connect("127.0.0.1:8080", SOCK_STREAM)` to attack local developer servers.
- **Pass Verification:**
  - In `Strict` mode (`CLONE_NEWNET`), every connection attempt must fail with `ENETUNREACH`, `ENETDOWN`, or `EADDRNOTAVAIL`.
  - Zero network packets may leave the container.

---

### ADV-04: Memory Allocation Bomb (Host OOM Denial of Service)
- **Threat Model:** The candidate attempts to allocate 4 GB of virtual memory in a tight loop to trigger host kernel out-of-memory swapping or crash the compiler parent process.
- **Attack Payload:**
  ```c
  while (1) {
      char *p = malloc(1024 * 1024 * 100); // 100 MB chunks
      if (p) memset(p, 0xAA, 1024 * 1024 * 100); // dirty pages
  }
  ```
- **Configured Limit:** `memory_allocation_ceiling = 128 MB`.
- **Pass Verification:**
  - The process is terminated by the kernel (Linux cgroup OOM / Windows Job Limit).
  - The supervisor reports `SandboxError::MemoryExhausted`.
  - The host machine experiences zero degradation, zero swap thrashing, and the parent compiler process remains unaffected.

---

### ADV-05: Fork Bomb (PID Table Exhaustion)
- **Threat Model:** The candidate executes a fork bomb `:(){ :|:& };:` to exhaust available PIDs on the host system.
- **Attack Payload:**
  ```c
  while (1) { fork(); }
  ```
- **Configured Limit:** `max_pids = 16`.
- **Pass Verification:**
  - Cgroups `pids.max` (Linux) or `JOB_OBJECT_LIMIT_ACTIVE_PROCESSES` (Windows) prevents fork proliferation past 16 processes.
  - New `fork()` calls fail immediately with `EAGAIN`.
  - The supervisor successfully terminates the container in $< 500\text{ ms}$ upon timeout without host PID exhaustion.

---

### ADV-06: Inherited File Descriptor Hijacking
- **Threat Model:** The parent compiler process opens a sensitive temporary file (e.g. FD 3 containing an API key). If file descriptors are not sanitized across `execve()`, the child reads FD 3.
- **Attack Payload:**
  ```c
  char buf[256];
  int n = read(3, buf, sizeof(buf)); // probe FD 3
  if (n > 0) { write(1, buf, n); }
  ```
- **Pass Verification:**
  - `close_range(3, ~0U, 0)` closes all parent FDs.
  - The child's `read(3, ...)` returns `-1` with `EBADF` (Bad File Descriptor).
  - Captured stdout contains zero bytes from FD 3.

---

### ADV-07: Unsafe `io_uring` Syscall Evasion
- **Threat Model:** Modern malware uses `io_uring` to bypass standard syscall interception frameworks, performing asynchronous network and file I/O.
- **Attack Payload:**
  ```c
  syscall(SYS_io_uring_setup, 32, &params);
  ```
- **Pass Verification:**
  - Seccomp-BPF filter traps `io_uring_setup`, `io_uring_enter`, and `io_uring_register`.
  - Syscall returns `-1` with `EPERM`.

---

### ADV-08: Stream Flooding & Bounded Shutdown
- **Threat Model:** The candidate floods stdout and stderr with 100 MB of continuous data to exhaust supervisor memory or cause pipe deadlocks.
- **Attack Payload:**
  ```c
  while (1) { write(1, "RUNAWAY_STREAM_DATA\n", 20); }
  ```
- **Configured Limit:** `stdout_limit_bytes = 64 KB`, `terminate_on_overflow = true`.
- **Pass Verification:**
  - Stream reader sets `overflow_detected = true` after 64 KB.
  - Supervisor sends `SIGKILL` to process tree.
  - Supervisor joins reader threads within $< 500\text{ ms}$ without deadlock.
  - Captured stdout length is strictly bounded to $\le 64\text{ KB}$.

---

### ADV-09: Fail-Closed Strict Invariant Enforcement
- **Threat Model:** A developer invokes `syn experiment` in `Strict` mode on a platform where full kernel unsharing is unavailable (e.g. native macOS Darwin without hypervisor).
- **Pass Verification:**
  - Supervisor inspects `PlatformCapabilities`.
  - Identifies that `filesystem_mount_confinement` and `network_egress_blocking` are `Unavailable`.
  - **Refuses to execute** without spawning any process.
  - Returns `Err(SandboxError::StrictInvariantViolation)`.

---

### ADV-10: Virtual Filesystem Masking Escape
- **Threat Model:** The candidate attempts to write to kernel tunables in `/proc/sys/kernel/core_pattern` or inspect host processes via `/proc`.
- **Attack Payload:**
  ```c
  int fd = open("/proc/sys/kernel/core_pattern", O_WRONLY);
  ```
- **Pass Verification:**
  - `/proc` inside the container is private to the container PID namespace.
  - `/proc/sys` is mounted read-only (`MS_RDONLY`).
  - Open for write returns `-1` with `EROFS` (Read-only file system).

---

## 4. Adversarial Test Execution Harness Matrix

| Test Identifier | Test Function in Suite | Target Invariant Verified | Expected Outcome |
| :--- | :--- | :--- | :--- |
| **ADV-01** | `test_adversarial_double_fork_daemon` | Process Tree Immunity | All processes killed (`ESRCH`) |
| **ADV-02** | `test_adversarial_host_credential_access` | Mount Confinement | `ENOENT` / `EACCES` on `~/.ssh` |
| **ADV-03** | `test_adversarial_network_egress_probe` | Network Namespace | `ENETUNREACH` on TCP/UDP |
| **ADV-04** | `test_adversarial_memory_allocation_bomb` | Memory Ceiling | `MemoryExhausted` / Process reaped |
| **ADV-05** | `test_adversarial_fork_bomb_proliferation`| PID Table Ceiling | `EAGAIN` / Killed in $< 500\text{ ms}$ |
| **ADV-06** | `test_adversarial_inherited_fd_sanitization`| FD Sanitization | `EBADF` on non-stdio descriptors |
| **ADV-07** | `test_adversarial_io_uring_syscall_blocked` | Seccomp Filter | `EPERM` on `io_uring_setup` |
| **ADV-08** | `test_adversarial_stream_flood_and_drain` | Stream Bounding | Bounded $\le 64\text{ KB}$, no deadlock |
| **ADV-09** | `test_adversarial_strict_mode_fail_closed`| Fail-Closed Policy | `StrictInvariantViolation` returned |
| **ADV-10** | `test_adversarial_procfs_masking_readonly`| Virtual FS Mask | `EROFS` on `/proc/sys` write |

---

## 5. Phase 2 Gate Criteria

Before any code in `crates/sandbox` can be signed off or **FIND-002** closed:
1. All 10 adversarial tests (**ADV-01** through **ADV-10**) must pass cleanly.
2. Full workspace tests (118 existing tests) must pass without regression.
3. Clippy must report 0 warnings with `-D warnings`.
4. Formatting must be 100% compliant.

*Deliverable 04 specification is complete. Implementation will proceed only after review and approval.*
