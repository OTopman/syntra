# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 09B: Stage 7 Corrective Specification & Multi-OS Acceptance Gate

**Audited Revision:** `39a1e955fd70180d86613d1cdc4af474c8af3bd7`  
**Resolves:** Findings A–E (`AUD-01` through `AUD-08` from Deliverable `09A`)  
**Date:** 2026-11-24  
**Status:** **CORRECTIVE SPECIFICATION — CODE FREEZE REMAINS ACTIVE UNTIL AUTHORIZED**

---

## 1. Purpose & Rule: Audit First, Correct Second, Test Third

In strict accordance with Stage 7 governance, **no production edits are performed prior to review of `09A` and authorization of these corrections**. This document specifies exact, testable fixes for every defect (`AUD-01` through `AUD-08`) and sets non-negotiable multi-OS acceptance gates before `FIND-002` may be closed.

---

## 2. Exact Corrective Specifications (`AUD-01` through `AUD-08`)

### 2.1 Fix `AUD-01`: Strict Fail-Closed Cgroup v2 Configuration (`syntra_sandbox_supervisor.rs`)
- Replace all `let _ = fs::write(...)` on cgroup controls with checked writes and read-back verification:
  - `memory.max` -> verify exact written integer matches.
  - `memory.swap.max` -> `write("0")?` (in Strict mode, missing or unwritable `memory.swap.max` MUST fail closed immediately).
  - `memory.oom.group` -> `write("1")?` (fail closed if unsupported/unwritable in Strict mode).
  - `pids.max` -> `write(pids_max)?`.
  - `cpu.max` -> `write("{quota_us} 100000")?` (fail closed on error in Strict mode).
- Remove cgroup directory on any partial configuration error before exiting `126`.

### 2.2 Fix `AUD-02`: Fail-Closed Allowlist Rootfs, `/dev` Nodes, RO Rootfs & Bounded `/tmp` (`syntra_sandbox_supervisor.rs`)
1. Propagate every `create_dir_all`, `fs::copy`, `chmod(0o555)`, `mount`, `pivot_root`, `chdir`, `umount2`, and `remove_dir` error strictly via `Result<(), String>`. Zero ignored `let _ =` calls on security-critical FS ops.
2. Before `pivot_root`:
   - Mount `/tmp` inside newroot as an independent quota-bounded `tmpfs` with `MS_NOSUID | MS_NODEV | MS_NOEXEC` (`size=16M,nr_inodes=512`).
   - Populate `/dev` as `tmpfs` (`nosuid,noexec,mode=0755,size=64k,nr_inodes=64`) and create + bind-mount ONLY individual safe virtual character device files (`/dev/null`, `/dev/zero`, `/dev/urandom`) using `O_NOFOLLOW` containment checks (`MS_BIND | MS_NOSUID | MS_NOEXEC`).
   - For each `ro_lib` in `spec.ro_libs`: validate via `O_NOFOLLOW` / canonical path check against allowlist prefixes, create mountpoint, bind-mount (`MS_BIND | MS_RDONLY | MS_NODEV | MS_NOSUID`), and fail closed on any error.
3. After `pivot_root` + `umount2("/.pivot_old", MNT_DETACH)` (returning `Err` if detach fails!):
   - Mount `/proc` (`MS_NOSUID | MS_NODEV | MS_NOEXEC`) and strictly verify `MS_RDONLY` over-mount of `/proc/sys` AND `/proc/sysrq-trigger` (aborting fail-closed if any mount fails).
   - Re-mount root `/` read-only (`MS_REMOUNT | MS_RDONLY | MS_NOSUID | MS_NODEV`) while `/tmp` remains the sole bounded writable location.
4. Now `/dev/null` exists cleanly inside the pivoted container so Tier 2's `open("/dev/null", O_RDONLY)` succeeds and strictly validates `dup2` return value (`>= 0`).

### 2.3 Fix `AUD-03`: Empirical Netlink Network Namespace Audit (`syntra_sandbox_supervisor.rs`)
- Eliminate the fail-open `/sys/class/net` check (`return true` on read error).
- Run a direct Netlink (`AF_NETLINK`, `NETLINK_ROUTE`) socket probe inside Trusted PID 1 (after entering `CLONE_NEWNET` and *before* Tier 2 installs Seccomp):
  1. Send `RTM_GETLINK` (`NLM_F_REQUEST | NLM_F_DUMP`) and parse returned `ifinfomsg` records:
     - Verify only `lo` interface exists (`IFLA_IFNAME == "lo"` or count == 1).
     - Verify `(ifi_flags & IFF_UP) == 0` (loopback strictly DOWN).
  2. Send `RTM_GETROUTE` (`NLM_F_REQUEST | NLM_F_DUMP`) and verify 0 routes exist across main/default routing tables.
  3. Close Netlink socket before `fork()` of Tier 2.
  4. If the Netlink socket or any check fails, return `Err` and fail closed (never write `net_audit=verified`).

### 2.4 Fix `AUD-04`: Corrected Seccomp-BPF Offsets, Complete FD Sweep & Tier 1 Error Propagation
1. **Fix BPF Jump Offsets:**
   - Replace hand-calculated jump offsets with explicit index calculation validated by unit-tested jump offsets (`target_idx - current_idx - 1`):
     - `sys_socket` mismatch jumps over `LD args[0]` + 4 domain checks (`5` instructions -> lands cleanly on `SECCOMP_RET_ALLOW`).
     - Each `deny_syscalls[i]` match jumps over remaining `(N - 1 - i)` `deny_syscalls` + `sys_socket` + `LD` + 4 domain checks + `RET_ALLOW` = `(N - 1 - i) + 7` instructions (verify exact instruction layout via unit test verifying Seccomp BPF assembly on all platforms!).
2. **Unbounded FD Close Before `execve`:**
   - Attempt `libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, libc::CLOSE_RANGE_CLOEXEC)`. If `ENOSYS`, enumerate `/proc/self/fd` inside isolated `/proc` (reading numeric FD names without heap allocation, or collecting before `execve` since `exec_err_w` is `O_CLOEXEC`) and mark every FD `>= 3` with `FD_CLOEXEC`, failing closed if `/proc/self/fd` cannot be opened.
3. **Structured Setup Error Channel (`Tier 1 -> Tier 0`):**
   - Distinguish infrastructure/policy setup errors at States S1–S7 (exit `126` with structured `setup_error=...` written to `tier1_evidence.txt`) from actual candidate exit codes, so `LinuxProvider` maps setup/policy failures directly to `Err(SandboxError::PlatformSetupFailure)` or `NetworkBoundaryCompromised`.

### 2.5 Fix `AUD-05`: Verifiable Windows Job Object Teardown (`platform/windows.rs`)
1. Check `TerminateJobObject` return value (`!= 0`); check `WaitForSingleObject` return value (`WAIT_OBJECT_0`, never silently ignore `WAIT_TIMEOUT`).
2. Inside `verify_and_cleanup()`:
   - Call `QueryInformationJobObject(h_job, JobObjectBasicAccountingInformation, ...)` AND `JobObjectBasicProcessIdList` to observe actual `ActiveProcesses` and remaining PIDs.
   - If `ActiveProcesses > 0` or `NumberOfProcessesInList > 0`, return `Err(SandboxError::TeardownFailure(...))` with the actual `surviving_pids` populated!
3. Redirect `hStdInput` to an explicit `CreateFileW(L"NUL", GENERIC_READ, ...)` handle (`bInheritHandle = TRUE`) included in `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`.

### 2.6 Fix `AUD-06`: Honest Linux Host Probing (`capabilities.rs`)
1. `probe_linux_host()` must test `unshare(CLONE_NEWUSER | CLONE_NEWNET | CLONE_NEWNS | CLONE_NEWPID)`.
2. `probe_linux_cgroup_v2_controllers()` must read `test_parent.join("cgroup.controllers")` (the actual delegated parent cgroup) and attempt creating/writing/deleting `memory.max`, `memory.swap.max`, `memory.oom.group`, `pids.max`, and `cpu.max` on a probe subdirectory rather than only checking `.exists()`.
3. `probe_clone_into_cgroup_flag()` must execute an empirical `clone3` probe (inside a short child/waitpid probe with invalid/noop flags or `CLONE_INTO_CGROUP` test) rather than inferring support solely from `/proc/sys/kernel/osrelease`.

### 2.7 Fix `AUD-07` & `AUD-08`: Supervisor Windows Drain, macOS FD Sweep, and Engine Overflow Check
1. **Windows `supervise_generic`:** Drive reader polling iteratively with `kill_tree()` invoked immediately if either stream signals overflow before `wall_clock_deadline`, checking overflow rather than waiting passively until timeout.
2. **Supervisor Error Propagation (`supervisor.rs`):** Propagate `child.kill_tree()?` failure cleanly (never swallow with `let _ =`).
3. **macOS FD Upper Bound (`macos.rs`):** Query `libc::getrlimit(libc::RLIMIT_NOFILE, &mut rl)` in parent prior to `spawn()`, pass `rl.rlim_cur.min(65536)` to `pre_exec` (falling back safely, or checking exact host `NOFILE`) so FDs `>= 4096` do not leak.
4. **SECE Engine (`lib.rs`):** Check `runner.last_stream_overflow()` inside `ExperimentalCompilationEngine::run_experiment` and mark any overflowing candidate as failed verification (`differential_equivalence_passed: false`).

---

## 3. Mandatory Multi-OS Acceptance Gate Before Closing `FIND-002`

Stage 7 fixes cannot be signed off or `FIND-002` closed until all four requirements below pass empirically:
1. **Corrected Code Review:** All fixes (`AUD-01`–`AUD-08`) implemented and unit-tested (including BPF bytecode offset verification unit tests runnable cross-platform!).
2. **Native Linux Execution Proof:** Compiled and executed on a Linux host with cgroups v2 delegation and user namespaces, passing `cargo test --workspace` + all expanded `ADV-01`–`ADV-10` and `FLT-01`–`FLT-10` tests (specifically including heavy-parent `ADV-04D`, Seccomp `ADV-07`, allowlist FS `ADV-02`, and netlink `ADV-03`) with archived JSON `ExecutionEvidenceReport` logs.
3. **Native Windows Execution Proof:** Compiled and executed on a Windows host, passing `cargo test -p syntra_sandbox` + `WindowsProvider` suspended Job Object, `ActiveProcesses == 0` teardown proof, and Strict-mode fail-closed verification.
4. **macOS Regression Proof:** All existing and macOS adversarial/fault-injection tests continue passing cleanly with 0 Clippy warnings and clean formatting.
