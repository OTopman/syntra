# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 08D: Windows & macOS Platform Security Contracts

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 Corrective Package (Resolves Finding 5, Review §2.D; `RT-05`, `RT-10`; `IND-06`)  
**Date:** 2026-11-24  
**Status:** **CORRECTED PLATFORM CONTRACTS — SUPERSEDES `06B` & `07B` WINDOWS/MACOS SECTIONS**

---

## 1. Fundamental Architectural Distinction: Resource Containment vs. Security Containment

Deliverable `07A`/`07B` treated Windows Job Objects as closely analogous to a Windows sandbox. In strict security engineering, **Resource Containment** and **Security Containment** are orthogonal:

- **Resource Containment:** Restricting memory commitments, active process count, CPU time, and termination propagation (`JOB_OBJECT_LIMIT_*`, `setrlimit`, timeouts).
- **Security Containment:** Preventing unauthorized read/write/connect operations against host files, registry hives, IPC endpoints, COM/RPC servers, named pipes, and networks.

**Rule:** Neither Windows Job Objects nor macOS POSIX process groups provide Security Containment. Advertising either as a Strict sandbox is forbidden.

---

## 2. Windows Platform Contract (`platform/windows.rs`)

### 2.1 Windows Resource Containment Provider (Job Object Engine)
When executing under `Guarded` mode (or as the resource layer of a future Windows Security Isolation Provider), the Windows Job Object implementation must obey every condition below:

1. **Creation & Checked Return Codes:**
   - Call `CreateJobObjectW(NULL, NULL)`; check non-null handle and capture `GetLastError()` on failure.
   - Configure `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`:
     - `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
     - `JOB_OBJECT_LIMIT_JOB_MEMORY` (aggregate committed virtual memory across the Job)
     - `JOB_OBJECT_LIMIT_ACTIVE_PROCESSES` (aggregate process count across the Job)
     - Explicitly **omit** `JOB_OBJECT_LIMIT_BREAKAWAY_OK` and `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`
   - Set UI/Desktop restrictions via `JOBOBJECT_BASIC_UI_RESTRICTIONS` (`JOB_OBJECT_UILIMIT_DESKTOP`, `EXITWINDOWS`, `HANDLES`, `READCLIPBOARD`, `SYSTEMPARAMETERS`, `WRITECLIPBOARD`) where applicable.
   - Check the boolean return value of **every** `SetInformationJobObject` call; abort cleanly on `0` (`FALSE`).
2. **Pre-Execution Suspended Spawning & Handle Sanitation:**
   - Allocate explicit `STARTUPINFOEXW` with `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` listing **only** the sanitized pipe handles for stdout/stderr (plus `NUL` for stdin if non-piped), passing `bInheritHandles = TRUE` strictly scoped to the attribute list OR `bInheritHandles = FALSE` where pipes are explicitly duplicated/configured—preventing unintended parent handle inheritance.
   - Invoke `CreateProcessW` with `CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP | EXTENDED_STARTUPINFO_PRESENT`.
   - Call `AssignProcessToJobObject(h_job, pi.hProcess)` while the initial thread has executed **0 instructions** (since Microsoft documentation explicitly states that allocations/actions prior to `AssignProcessToJobObject` are not charged against or confined by the Job).
   - **Nested Job Handling:** On Windows 8 / Server 2012+, nested jobs are supported. However, if the compiler itself runs inside an existing parent Job Object (e.g. CI runner) that enforces `JOB_OBJECT_LIMIT_BREAKAWAY_OK` disabled or conflicting ceilings where `AssignProcessToJobObject` fails (`ERROR_ACCESS_DENIED`), the provider **must fail closed** rather than continuing outside a Job!
   - Call `ResumeThread(pi.hThread)` and **check its return value** (`(DWORD)-1` indicates failure -> immediately call `TerminateProcess`, close all handles, and fail closed).
3. **Deterministic Handle Cleanup:**
   - Wrap every Win32 `HANDLE` (`h_job`, `h_process`, `h_thread`, pipe handles) in RAII `OwnedHandle` guards enforcing `CloseHandle`. Closing `h_job` triggers `KILL_ON_JOB_CLOSE`.

### 2.2 Windows Security Containment Gap (`IND-06`) & Strict Refusal
Even with Job Objects + `CREATE_SUSPENDED`, Windows Job Objects alone have **known escape paths and missing boundaries**:
- **Missing Filesystem Isolation:** Candidate can read developer secrets (`%USERPROFILE%\.ssh`, `.env`, browser/credential stores).
- **Missing Registry & Named Pipe Isolation:** Candidate can read/modify HKCU registry keys or connect to local RPC/COM/named pipes.
- **Missing Network Isolation:** Candidate can open Winsock TCP/UDP sockets to external C2 hosts or `127.0.0.1`.
- **Out-of-Process Spawn Escape (`IND-06`):** As documented by Microsoft, processes spawned indirectly through out-of-process DCOM/WMI/RPC brokers (such as `Win32_Process.Create` via WMI or Task Scheduler COM activation) are created by the host broker service (`WmiPrvSE.exe` / `svchost.exe`), **NOT** as children of the candidate process, and therefore **do not join the candidate's Job Object**. Preventing this requires stripping IPC/COM privileges via Restricted Tokens / Low Integrity MIC / AppContainer + Firewall isolation.

### 2.3 Required Windows Security Isolation Provider Boundary
- A separate `WindowsSecurityProvider` interface (targeting Windows **AppContainer** + **Restricted Token / Integrity Level** + **Windows Filtering Platform / Firewall rules** or Hyper-V/Container isolation) is required before Windows can satisfy `Strict` mode.
- **Current Enforcement Rule (Non-Negotiable):** Until a dedicated Windows Security Isolation Provider is implemented and adversarially verified, **all `Strict` capabilities for filesystem confinement, network egress blocking, IPC containment, and host identity restriction on native Windows report `Unsupported` / `Unavailable`**.
- Attempting `EnforcementMode::Strict` on native Windows **MUST FAIL CLOSED** with `SandboxError::StrictInvariantViolation`, spawning zero processes.
- `EnforcementMode::Guarded` is permitted solely for trusted local developer experiments, emitting an explicit banner documenting that only Job Object **Resource Containment** is active and host filesystem/network/COM-broker escapes are **not** blocked.

---

## 3. macOS Platform Contract (`platform/macos.rs`)

### 3.1 Available Supported Primitives on Darwin / XNU
Only the following controls are available via supported, non-deprecated unprivileged macOS POSIX/Darwin APIs:
- New process group via `setpgid(0, 0)` (note: vulnerable if candidate calls `setsid()`).
- Best-effort group termination via `kill(-pgid, SIGKILL)`.
- Wall-clock deadline enforcement via supervisor `poll()` + `waitpid`.
- Bounded non-blocking stdout/stderr capture (`O_NONBLOCK` + monotonic deadline drain).
- Environment sanitization (`env_clear()`), `stdin` bound to `/dev/null`, and `FD_CLOEXEC` / `close` sweep on host FDs > 2.
- CPU time limit via `setrlimit(RLIMIT_CPU)` (note: `RLIMIT_AS` is non-enforcing / unreliable for physical memory ceiling on XNU).

### 3.2 Explicitly Unsupported Guarantees on macOS
The following are **explicitly `Unsupported` / `Unavailable`** on native macOS without external virtualization:
- Filesystem mount isolation (no unprivileged mount namespaces; `sandbox-exec` is deprecated and unsupported by Apple; AppSandbox requires code signing/entitlements unsuitable for unverified compiler candidates).
- Network egress blocking (no unprivileged network namespaces).
- Immune process-tree termination (any candidate calling `setsid()` escapes `kill(-pgid)`).
- Reliable aggregate memory & swap accounting (`RLIMIT_AS` and `RLIMIT_DATA` do not establish cgroup-equivalent tree containment on XNU).

### 3.3 Strict vs. Guarded Policy on macOS
- **`EnforcementMode::Strict`:** Unconditionally **REFUSED (Fail-Closed)** with `SandboxError::StrictInvariantViolation` prior to spawning any process.
- **`EnforcementMode::Guarded`:** Permitted **only** when explicitly enabled by a developer for trusted local code (`--guarded`). Must emit an unmistakable stderr security warning stating that filesystem, network, memory-tree, and `setsid()`-immune process-tree containment are **absent**, and teardown/cleanup (`waitpid`, temp dir removal, bounded stream drain) must still execute cleanly (`ADV-09B`).

---

## 4. Unsupported / Fallback Provider (`platform/fallback.rs`)

- For any OS outside Tier 1 Linux, Tier 2 Windows, or Tier 2 macOS:
  - Every security invariant reports `Unsupported`.
  - Both `Strict` mode and unverified execution fail closed immediately without spawning.
