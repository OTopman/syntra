# Syntra Phase 2: SECE Sandbox Security Architecture
# Deliverable 02: Threat Model & Multi-OS Architecture Specification

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox  
**Target Milestone:** Phase 2 Security Architecture  
**Author:** Principal Compiler Security Engineer, Systems Runtime Architect & Supply-Chain Auditor  
**Date:** 2026-11-24  
**Status:** **ARCHITECTURAL SPECIFICATION — APPROVED FOR REVIEW**

---

## 1. Executive Charter & Core Security Principles

The Phase 2 security architecture defines the execution boundaries for the **Syntra Autonomous Experimental Compilation Engine (SECE)**. SECE compiles and runs speculative optimization variants and untrusted candidate code.

To eliminate the historical "pseudo-sandbox" vulnerability (**FIND-002**), the SECE sandbox is built upon four non-negotiable architectural principles:

### Principle 1: Fail-Closed Security
If the host operating system or environment cannot provide the required isolation guarantees, **Syntra must refuse to execute untrusted candidates**. It must never silently fall back to ordinary, unconfined host execution.

### Principle 2: Capability-Based Isolation
Every platform backend must programmatically report its verified isolation capabilities. Surface-level mechanisms (such as setting environment variables or clearing strings) must **never** be conflated with filesystem, network, or process isolation.

### Principle 3: Single Abstract Security Contract (`SandboxContract`)
Linux, macOS, and Windows implement the same unified sandbox contract. The public compiler and optimization engine interfaces interact solely with the contract, while platform-specific providers enforce native kernel primitives.

### Principle 4: Zero Trust in Candidate Code
Candidate machine code is treated as potentially adversarial. Optimization passes may introduce memory-safety bugs, dependencies may harbor supply-chain payloads, and runaway code may attempt host resource starvation.

---

## 2. Threat Model & Attacker Profile

```
┌───────────────────────────────────────────────────────────────────────────┐
│                        UNTRUSTED EXECUTION ZONE                           │
│  Candidate Native Binary (Compiled AIR / Cranelift / Linked Executable)   │
│  Attacker Goals: Exfiltrate Secrets | Escape Sandbox | Host Denial of Svc │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │ Syscalls / I/O Attempts
                                      ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                        SANDBOX MEDIATION BOUNDARY                         │
│  Process-Group Containment │ Namespaces │ Quotas │ Syscall Filters        │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │ Enforced Limits & Isolations
                                      ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                          TRUSTED HOST ZONE                                │
│  Developer Home (~/.ssh, ~/.aws) │ Workspace Source Code │ Network │ OS   │
└───────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Attacker Objectives & Capabilities
The threat model assumes an attacker has achieved code execution within the candidate process—either via an autonomous optimization failure, an unverified library dependency, or intentionally malicious Syntra source code.

The attacker seeks to:
1. **Exfiltrate Developer Credentials:** Read sensitive host files (`~/.ssh/id_rsa`, `~/.aws/credentials`, `~/.gnupg`, `.env`, tokens in parent directories).
2. **Exfiltrate Data via Network:** Initiate outbound TCP/UDP connections, make HTTP requests, or exfiltrate host data via DNS queries.
3. **Establish Persistence on Host:** Write to autostart scripts (`~/.bashrc`, `~/.zshrc`), install scheduled tasks (`cron`, systemd user services, `launchd`), or overwrite toolchain binaries.
4. **Denial of Service (DoS):** Exhaust host memory (triggering kernel OOM crashes), spawn fork-bombs to exhaust PID tables, or flood disk storage with unbounded log writes.
5. **Evade Termination via Daemons:** Fork, double-fork, or call `setsid()` to detach child worker processes that persist after the parent runner process terminates.
6. **Kernel & Syscall Exploitation:** Exploit host kernel vulnerabilities via dangerous syscalls (`ptrace`, `bpf`, `mount`, `io_uring`).

---

## 3. The Syntra Sandbox Security Contract (`SandboxContract`)

Every platform implementation must satisfy or explicitly negotiate the five core contract invariants:

### Invariant 1: Filesystem Confinement
- The candidate must execute with an **isolated, ephemeral root or strictly filtered directory view**.
- **No access** to developer home directories (`~/*`), SSH keys, cloud credentials, or sensitive system files.
- The candidate can only write to a private, ephemeral directory that is automatically purged upon completion or termination.
- Host source files outside the package root are invisible or strictly unreadable.

### Invariant 2: Network Confinement
- The candidate must have **zero outbound network connectivity** (no TCP, no UDP, no raw sockets).
- Loopback access (`127.0.0.1`, `::1`) is disabled or isolated to prevent probing host services (databases, internal APIs, Docker sockets).
- DNS resolution must fail immediately without host leakage.

### Invariant 3: Process Tree Confinement & Orphan Prevention
- The candidate process and **all of its descendant processes** (children, grandchildren, detached daemons) must be contained within a single tracked process group or PID namespace.
- Upon completion, failure, or timeout, **every process in the tree must be terminated with `SIGKILL`**.
- Orphaned or zombie processes must never survive runner execution.

### Invariant 4: Hard Resource Quotas
- **Memory Ceiling:** A hard memory quota (e.g. 512 MB RSS) enforced by the kernel. Exceeding the quota triggers immediate process termination without impacting host processes.
- **CPU Time & Core Quotas:** Candidates are bounded to specific execution time limits (CPU time and wall-clock timeout) and limited core counts.
- **Process Count Limit:** Maximum process count limit (`max_pids`, e.g. 16) to completely neutralize fork-bombs.
- **Disk Write Quotas:** Ephemeral directory write size capped (e.g. 10 MB) to prevent disk exhaustion.

### Invariant 5: Syscall Surface Minimization
- Dangerous syscalls are blocked at the kernel boundary: `ptrace`, `process_vm_readv`, `mount`, `chroot`, `reboot`, raw network sockets, and kernel module loading.

---

## 4. Multi-OS Architecture & Platform Capability Strategy

Different operating systems offer fundamentally different kernel isolation primitives. Syntra adopts a **Tiered Capability Architecture** that never obscures platform differences.

```
┌───────────────────────────────────────────────────────────────────────────┐
│                      SandboxPlatformProvider (Trait)                       │
│  • capabilities() -> PlatformCapabilities                                 │
│  • spawn_isolated(config) -> Box<dyn IsolatedProcess>                     │
│  • enforce_termination(handle) -> Result<(), SandboxError>                │
└───────────────────┬───────────────────┬───────────────────┬───────────────┘
                    │                   │                   │
         ┌──────────┴────────┐ ┌────────┴────────┐ ┌────────┴────────┐
         │  LinuxProvider    │ │  MacOSProvider  │ │ WindowsProvider │
         │  (Tier 1: Full)   │ │  (Tier 2: Proc) │ │ (Tier 2: Job)   │
         └───────────────────┘ └─────────────────┘ └─────────────────┘
```

---

### 4.1 Linux Platform Provider (Tier 1 — Full Containerless Isolation)

Linux provides the most robust native containerless isolation primitives in standard userland.

#### Architecture:
1. **User Namespaces (`CLONE_NEWUSER`):** Maps the current user to a single UID (`root` inside the namespace, unprivileged host UID outside). Allows unprivileged sandbox setup without `sudo`.
2. **Mount Namespaces (`CLONE_NEWNS`) & `pivot_root`:**
   - Mounts a minimal `tmpfs` as new `/`.
   - Read-only bind-mounts minimal system runtime libraries (`/lib`, `/lib64`, `/usr/lib`).
   - Private ephemeral `/tmp` and candidate directory.
   - Developer home directory (`/home/*`), `/etc/shadow`, `/root`, and parent directories do not exist in the mount table.
3. **Network Namespaces (`CLONE_NEWNET`):** Completely unshares the network stack. No network interfaces exist except an unconfigured loopback (`lo`). Outbound sockets fail with `ENETUNREACH`.
4. **PID Namespaces (`CLONE_NEWPID`):** The candidate process is PID 1 in its namespace. Spawning child processes is strictly contained. When PID 1 is terminated, the Linux kernel automatically destroys all descendant processes in the namespace.
5. **Cgroups v2:**
   - `memory.max`: Hard limit on candidate RSS memory.
   - `pids.max`: Hard limit (16 processes) preventing fork bombs.
   - `cpu.max`: Throttles runaway CPU consumption.
6. **Seccomp-BPF:** Restricts syscalls to minimal computation (`read`, `write`, `exit_group`, `nanosleep`, `brk`, `mmap`, `futex`). Rejects `socket`, `bind`, `connect`, `ptrace`, `kill`.

#### Fail-Closed Refusal:
If the host Linux kernel disables unprivileged user namespaces (`kernel.unprivileged_userns_clone = 0`), Syntra **refuses execution** in Strict mode.

---

### 4.2 macOS Platform Provider (Tier 2 — Hardened Process & Resource Boundary)

#### Architectural Decision: Rejection of Deprecated `sandbox-exec`
Earlier drafts proposed `sandbox-exec`. However:
- `sandbox-exec` is officially deprecated by Apple and may be removed in future macOS versions.
- Undocumented Seatbelt profiles (`(version 1) (deny default)...`) change without notice across macOS versions.

#### Robust macOS Strategy:
1. **Process Group & Session Isolation:**
   - Every candidate is spawned in a new process group and session (`posix_spawnattr_setpgroup(&attr, 0)`).
   - Process termination sends `SIGKILL` to the entire process group (`kill(-pgid, SIGKILL)`), ensuring forked children and grandchildren are reaped.
2. **POSIX Resource Limits (`setrlimit`):**
   - `RLIMIT_AS` / `RLIMIT_DATA`: Restricts virtual memory and heap allocations.
   - `RLIMIT_CPU`: Hard CPU time quota.
   - `RLIMIT_NPROC`: Maximum child processes per UID.
   - `RLIMIT_FSIZE`: Caps individual file write sizes.
3. **Isolated Ephemeral Workspaces:**
   - Candidate executes in an ephemeral directory under `/private/tmp/syntra_sandbox/<run_id>`.
   - Clears all environment variables (`.env_clear()`) and removes parent directory write permissions.
4. **Explicit Capability Reporting & Refusal:**
   - macOS explicitly reports:
     `{ filesystem_virtualization: false, network_unshare: false, process_tree_kill: true, memory_rlimit: true, cpu_rlimit: true }`.
   - In **Strict Mode**, Syntra will **refuse execution** on macOS unless run within an approved external container or until macOS virtualization backends (Hypervisor framework) are integrated.
   - In **Guarded Mode**, Syntra allows compilation with resource quotas and process-tree termination, warning the developer of missing kernel network/filesystem namespaces.

---

### 4.3 Windows Platform Provider (Tier 2 — Job Objects & Restricted Tokens)

#### Architectural Strategy:
1. **Windows Job Objects (`CreateJobObjectW`):**
   - Configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: Closing the job handle automatically terminates all active child, grandchild, and descendant processes.
   - `JOB_OBJECT_LIMIT_JOB_MEMORY`: Enforces a hard physical memory ceiling on the combined process tree.
   - `JOB_OBJECT_LIMIT_ACTIVE_PROCESSES`: Caps maximum concurrent processes (fork-bomb prevention).
2. **Restricted Security Tokens:**
   - Uses `CreateRestrictedToken` to strip administrative and developer privileges, disabling token elevation.
3. **Explicit Capability Reporting:**
   - Reports filesystem and network virtualization as unavailable natively without AppContainer / Windows Sandbox.
   - Refuses execution under strict isolation requirements.

---

## 5. Enforcement Policies & Capability Matrix

| Feature / Guarantee | Linux (Tier 1) | macOS (Tier 2) | Windows (Tier 2) |
| :--- | :---: | :---: | :---: |
| **Filesystem Mount Isolation** | **Enforced** (`CLONE_NEWNS`, `pivot_root`) | Partial (Directory sandbox) | Partial (Restricted token) |
| **Network Stack Disablement** | **Enforced** (`CLONE_NEWNET`) | *Requires Container* | *Requires AppContainer* |
| **Process Tree Termination** | **Enforced** (`CLONE_NEWPID`) | **Enforced** (`kill(-pgid)`) | **Enforced** (`JobObject`) |
| **Memory Ceiling (RSS)** | **Enforced** (cgroups v2) | **Enforced** (`setrlimit`) | **Enforced** (`JobMemoryLimit`) |
| **Fork-Bomb Prevention** | **Enforced** (`pids.max`) | **Enforced** (`RLIMIT_NPROC`) | **Enforced** (`ActiveProcessLimit`) |
| **Syscall Whitelisting** | **Enforced** (Seccomp-BPF) | *Requires Seatbelt* | N/A |
| **Default Security Status** | **FULL CONFINEMENT** | **GUARDED PROCESS** | **GUARDED PROCESS** |

### Execution Enforcement Modes:
1. **`EnforcementMode::Strict` (Default for Production & Untrusted Candidates):**
   - Requires Tier 1 full confinement (Filesystem + Network + Process + Quotas).
   - If the platform cannot guarantee network or filesystem unsharing, **execution is rejected with an explicit error**.
2. **`EnforcementMode::Guarded` (Developer Local Testing):**
   - Enforces process-tree termination, memory ceilings, CPU timeouts, and output limits.
   - Emits an explicit audit warning detailing non-enforced boundaries (e.g. host network access on macOS).

---

## 6. Execution Lifecycle State Machine

```
   [ Unconfigured ]
          │
          ▼
   [ Capability Audit ] ──(Missing required guarantees)──► [ EXECUTION REFUSED ]
          │                                                       (Fail-Closed)
          ▼ (Guarantees Verified)
   [ Provisioning Sandbox ]
          │ (Mounts, namespaces, limits, env)
          ▼
   [ Process Spawning ] (New session / process group / job)
          │
          ▼
   [ Supervised Execution ] ◄── Polling Loop (CPU, memory, output stream)
          │
    ┌─────┴────────────────────────┐
    ▼                              ▼
(Exceeded Quotas / Timeout)   (Normal Exit)
    │                              │
    ▼                              ▼
[ Tree Kill: SIGKILL -pgid ]  [ Verification & Output Collection ]
    │                              │
    └──────────────┬───────────────┘
                   ▼
         [ Purge Ephemeral Dir ]
                   │
                   ▼
         [ Experiment Report ]
```

---

## 7. Adversarial Escape Test Specification (Pre-Implementation Plan)

Before implementing the new sandbox abstractions, an adversarial test suite must be defined. Every proposed mechanism will be challenged with 8 dedicated attack scripts:

1. **`test_escape_double_fork_daemon`:** Spawns a background process that executes `setsid()` and attempts to survive runner exit. Must be killed.
2. **`test_escape_read_host_credentials`:** Attempts to open `/etc/shadow`, `~/.ssh/id_rsa`, and `~/.aws/credentials`. Must return `ENOENT` or `EACCES`.
3. **`test_escape_outbound_network`:** Attempts to open a TCP socket to `8.8.8.8:53` and probe `localhost:80`. Must return `ENETUNREACH` or fail immediately.
4. **`test_escape_memory_bomb`:** Allocates 2GB of virtual memory in a tight loop. Must be terminated by memory quota without host degradation.
5. **`test_escape_fork_bomb`:** Executes a shell fork bomb `:(){ :|:& };:`. Must be contained and terminated without exhausting host PIDs.
6. **`test_escape_disk_flooding`:** Writes 100MB of random bytes to disk. Must fail on write quota.
7. **`test_escape_dangerous_syscall`:** Attempts to invoke `ptrace(PTRACE_TRACEME)`. Must be blocked by Seccomp filter.
8. **`test_fail_closed_unsupported_platform`:** In `Strict` mode on a platform missing network isolation, execution must refuse to run and return `Err(SandboxRefusal)`.

---

## 8. Conclusion

Deliverable 02 establishes the formal specification for Phase 2:
- Replaces vague "sandbox" claims with a formal **Capability Contract**.
- Implements **Fail-Closed Security** (no silent fallbacks).
- Defines concrete, native OS mechanisms for Linux, macOS, and Windows.
- Establishes an 8-point adversarial test matrix prior to implementation.
