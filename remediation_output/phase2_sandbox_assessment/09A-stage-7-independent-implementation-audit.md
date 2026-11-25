# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 09A: Stage 7 Independent Source-Code Implementation Audit

**Audited Repository & Revision:** `OTopman/syntra` (`develop` @ `39a1e955fd70180d86613d1cdc4af474c8af3bd7`)  
**Local Working Tree Status:** Matches `origin/develop` cleanly (`nothing to commit, working tree clean`)  
**Audited Scope:** All Stage 7 files in `crates/sandbox/` compared line-by-line against approved contracts `08A`–`08H`  
**Audit Date:** 2026-11-24  
**Stage 7 Completion Verdict:** **FAIL / WITHHELD — CRITICAL IMPLEMENTATION DEFECTS & UNVERIFIED CROSS-OS CONTROLS**

---

## 1. Executive Summary & Audit Verdict

Following the Stage 7 implementation commit (`39a1e955fd70180d86613d1cdc4af474c8af3bd7`), this document provides an independent, line-by-line source audit comparing the actual committed Rust code in `crates/sandbox/` against the approved security specifications (`08A`–`08H`).

### Core Governance Determination:
1. **Production Code Freeze Re-Established:** No further feature work or code edits were made during this audit.
2. **macOS Host Regression Baseline (`128/128`) Is Not Multi-OS Proof:** The 128 passing tests observed on `x86_64-apple-darwin` confirm strict fail-closed refusal and cooperative Guarded controls on macOS; they do **not** compile or test Linux (`#[cfg(target_os = "linux")]`) or Windows (`#[cfg(target_os = "windows")]`) kernel enforcement.
3. **Line-by-Line Inspection Uncovered Critical Implementation Defects:** Independent code inspection identified **8 concrete source-level findings (`AUD-01` through `AUD-08`)**, confirming all five preliminary issues (**Findings A–E**) plus additional critical Seccomp-BPF, `/dev/null` mount, Windows overflow, and Linux probe defects.

---

## 2. Repository & File Inventory Audited

| File | Revision Verified | Host Execution Status | Audit Outcome |
| :--- | :---: | :---: | :---: |
| [`crates/sandbox/src/error.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/error.rs) | `39a1e95` | Compiled & tested (macOS) | **PASS** |
| [`crates/sandbox/src/config.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/config.rs) | `39a1e95` | Compiled & tested (macOS) | **PASS** |
| [`crates/sandbox/src/traits.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/traits.rs) | `39a1e95` | Compiled (macOS) | **PASS** |
| [`crates/sandbox/src/capabilities.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/capabilities.rs) | `39a1e95` | macOS paths tested; Linux/Win **uncompiled** | **FAIL** (`AUD-06`: naive kernel probes) |
| [`crates/sandbox/src/supervisor.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/supervisor.rs) | `39a1e95` | Unix path tested (macOS); non-Unix **uncompiled** | **FAIL** (`AUD-07`: Windows overflow/drain, cleanup error masking) |
| [`crates/sandbox/src/platform/macos.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/platform/macos.rs) | `39a1e95` | Compiled & tested (macOS) | **PASS WITH DOCUMENTED LIMITATION** (`AUD-07`: 4096 FD bound, cooperative `setsid()` limit) |
| [`crates/sandbox/src/platform/linux.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/platform/linux.rs) | `39a1e95` | **UNCOMPILED & UNTESTED** | **FAIL** (`AUD-04`: premature exit handling & optimistic capability mapping) |
| [`crates/sandbox/src/bin/syntra_sandbox_supervisor.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/bin/syntra_sandbox_supervisor.rs) | `39a1e95` | Main stub only on macOS; Linux core **UNCOMPILED & UNTESTED** | **FAIL** (`AUD-01`, `AUD-02`, `AUD-03`, `AUD-04`: non-fail-closed cgroup/rootfs, fake netns audit, BPF jump bug, missing `/dev/null`) |
| [`crates/sandbox/src/platform/windows.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/platform/windows.rs) | `39a1e95` | **UNCOMPILED & UNTESTED** | **FAIL** (`AUD-05`: unverified Job teardown & swallowed kill errors) |
| [`crates/sandbox/src/platform/fallback.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/platform/fallback.rs) | `39a1e95` | Compiled (macOS) | **PASS** (unconditional fail-closed) |
| [`crates/sandbox/src/runner.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/runner.rs) & [`lib.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/lib.rs) | `39a1e95` | Compiled & tested (macOS) | **FAIL** (`AUD-08`: `ExperimentalCompilationEngine` ignores `last_stream_overflow`) |

---

## 3. Security Invariants Audit Matrix (`PASS` / `FAIL` / `UNVERIFIED`)

| # | Mandatory Security Invariant (`08G`) | Current Disposition | Governing Basis & Observed Evidence |
| :- | :--- | :---: | :--- |
| **1** | **No untrusted execution without verified isolation** | **FAIL / UNVERIFIED** | macOS default `Strict` fails closed (`ADV-09A` passes); however, Linux code emits `=verified` without failing closed on failed mounts/netns/cgroups (`AUD-01`–`AUD-04`) and is uncompiled/untested on Linux. |
| **2** | **Strict enforcement fails closed** | **FAIL** | macOS **PASS** (`ADV-09A`). Linux **FAIL**: `syntra_sandbox_supervisor.rs` ignores `memory.swap.max`, `memory.oom.group`, `cpu.max`, bind-mount, `/proc`, `/proc/sys`, and `/sys/class/net` read errors (`AUD-01`, `AUD-02`, `AUD-03`). |
| **3** | **Guarded execution reports actual enforcement guarantees** | **PASS (macOS) / UNVERIFIED (Win/Linux)** | Verified empirically on macOS (`ADV-09B`): stderr banner emitted and `ExecutionEvidenceReport` marks FS/Net `Unsupported` and process-tree `PartiallyEnforced`. Windows & Linux Guarded untested. |
| **4** | **Capability states avoid misleading booleans** | **FAIL** | Schema exists (`08E`), **PASS** on macOS, but Linux `capability_evidence()` accepts forged/optimistic flags from `tier1_evidence.txt` even when probes failed or skipped (`AUD-03`, `AUD-06`). |
| **5** | **Linux and macOS/Windows do not claim equivalent guarantees** | **PASS** | `capabilities.rs` separates Tier 1 Linux from Tier 2 macOS/Windows cleanly across types and evidence reports. |
| **6** | **Resource limits cover aggregate process-tree consumption** | **FAIL / UNVERIFIED** | Linux `memory.oom.group=1` and `cpu.max` writes ignore errors (`AUD-01`); Windows `kill_tree`/`verify_and_cleanup` do not verify Job process list empty (`AUD-05`); neither target compiled/executed. |
| **7** | **Memory and swap accounting explicitly addressed** | **FAIL / UNVERIFIED** | `memory.swap.max=0` error ignored (`AUD-01`), `clone3(CLONE_INTO_CGROUP)` inferred from kernel version string rather than runtime probe (`AUD-06`), and `ADV-04A..G` unexecuted on Linux. |
| **8** | **Process supervision resists PID reuse and lifecycle races** | **UNVERIFIED (Linux) / COOPERATIVE ONLY (macOS)** | Linux `pidfd` + `clone3(CLONE_PIDFD)` + `PR_SET_PDEATHSIG` implemented in source, **UNVERIFIED** (uncompiled/untested on Linux). macOS `setpgid` documented cooperative only (Finding E). |
| **9** | **Standard descriptors & terminal attachment sanitised** | **PASS (macOS, <4096) / FAIL (Linux, macOS >=4096)** | macOS `ADV-06` passes for FDs `< 4096`, but `clamp(256, 4096)` misses host FDs `>= 4096` (`AUD-07`). Linux pivoted rootfs lacks `/dev/null` node so `open("/dev/null")` fails silently (`AUD-02`). |
| **10**| **Output collection bounded and deadline-controlled** | **PASS (Unix) / FAIL (Windows)** | Unix `poll()` + 500 ms drain empirically passes (`ADV-08A`, `ADV-08B`). Windows `supervise_generic` spawns detached threads and omits immediate overflow kill (`AUD-07`). |
| **11**| **Process descendants do not survive sandbox termination** | **FAIL (Linux/Win) / COOPERATIVE (macOS)** | macOS `kill(-pgid)` runs unconditionally on exit/teardown, cooperative against `setsid()`. Windows does not query Job process list (`AUD-05`). Linux `cgroup.kill` uncompiled/untested. |
| **12**| **Cleanup deterministic, idempotent, and non-silent on error** | **FAIL** | `supervisor.rs` ignores `kill_tree()` error in Unix fallback (`line 125`), Windows `verify_and_cleanup` ignores `TerminateJobObject` error (`AUD-05`), Linux `cleanup_run` ignores `kill_cgroup_tree` errors (`line 128`). |

---

## 4. Finding-by-Finding Source Verification Matrix

### `AUD-01` (Preliminary Finding A — CRITICAL): Linux Cgroup Configuration Is Not Fail-Closed
- **Source Location:** [`syntra_sandbox_supervisor.rs#L62-L69`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/bin/syntra_sandbox_supervisor.rs#L62-L69)
- **Observed Code:**
  ```rust
  let _ = fs::write(cgroup_dir.join("memory.swap.max"), "0");
  let _ = fs::write(cgroup_dir.join("memory.oom.group"), "1");
  ...
  let _ = fs::write(cgroup_dir.join("cpu.max"), format!("{quota_us} 100000"));
  ```
- **Contract Violated:** `08B` §3 (State S1->S2) & `08C` §2 (`swap_elimination`, `cpu.max`, `memory.oom.group = 1`).
- **Security Impact:** If `memory.swap.max`, `memory.oom.group`, or `cpu.max` returns `EACCES`, `ENOENT`, or `EINVAL`, setup silently continues and executes the candidate without swap prevention, atomic group OOM termination, or CPU throttling while reporting `mem_cgroup=verified`.

### `AUD-02` (Preliminary Finding B — CRITICAL): Linux Filesystem Construction Ignores Errors & Breaks `/dev/null`
- **Source Location:** [`syntra_sandbox_supervisor.rs#L245-L341`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/bin/syntra_sandbox_supervisor.rs#L245-L341), [`#L467-L473`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/bin/syntra_sandbox_supervisor.rs#L467-L473)
- **Observed Code Defects:**
  1. Ignores `create_dir_all` for `rootfs`, `candidate`, `tmp`, `proc`, `dev`, `.pivot_old` (lines 260, 278–282).
  2. `ro_libs` loop silently skips failed `canonicalize`, directory creation, file creation, and ignores `libc::mount` failure (`let _ = libc::mount(...)` lines 291–311); lacks `O_NOFOLLOW`/`RESOLVE_BENEATH` containment checks.
  3. Ignores `umount2(c"/.pivot_old", MNT_DETACH)` failure (line 321), risking host root retention inside container.
  4. Ignores `libc::mount("proc", ...)` and `libc::mount("tmpfs", "/proc/sys", MS_RDONLY, ...)` errors (lines 325–338).
  5. Never mounts isolated writable `/tmp` (`noexec,nosuid,nodev,size=...`), never remounts rootfs `MS_RDONLY`, and never populates `/dev/null`, `/dev/zero`, `/dev/urandom` inside `/dev`! Consequently, Tier 2's `libc::open(c"/dev/null")` at line 467 fails (`ENOENT`) inside the pivoted root!
- **Contract Violated:** `08B` S4->S5, S6->S7; `08C` §3.1.

### `AUD-03` (Preliminary Finding C — CRITICAL): Network Isolation Audit Fails Open
- **Source Location:** [`syntra_sandbox_supervisor.rs#L343-L353`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/bin/syntra_sandbox_supervisor.rs#L343-L353)
- **Observed Code:**
  ```rust
  fn audit_netns_isolated() -> bool {
      let Ok(entries) = fs::read_dir("/sys/class/net") else {
          return true; // /sys intentionally unmounted in private pivot_root
      };
  ```
- **Contract Violated:** `03B` §2.1, `08B` S4->S5, `08E` §3.
- **Security Impact:** Because `setup_allowlist_rootfs()` omits `/sys`, `/sys/class/net` **never exists** when `audit_netns_isolated()` runs. It unconditionally returns `true` and records `net_audit=verified` without probing any network interface, `IFF_UP` loopback flag, or routing table.

### `AUD-04` (CRITICAL — Newly Discovered in Audit): Off-by-One Seccomp-BPF Jump Table & Lifecycle Propagation Bugs
- **Source Locations:**
  1. [`syntra_sandbox_supervisor.rs#L403-L445`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/bin/syntra_sandbox_supervisor.rs#L403-L445)
  2. [`syntra_sandbox_supervisor.rs#L214-L218`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/bin/syntra_sandbox_supervisor.rs#L214-L218)
  3. [`crates/sandbox/src/platform/linux.rs#L170-L183`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/platform/linux.rs#L170-L183)
- **Observed Code Defects:**
  1. **Seccomp BPF Off-by-One:** In `install_seccomp_filter()`, `sys_socket` check has `jf: 6` (lines 418–422). Counting instructions after `sys_socket`: `[0] LD`, `[1] AF_INET`, `[2] AF_INET6`, `[3] AF_NETLINK`, `[4] AF_PACKET`, `[5] RET ALLOW`, `[6] RET DENY`. Jumping `jf: 6` lands directly on `RET DENY`, denying ALL non-socket syscalls! Likewise, `jump_to_deny = (deny_idx_offset - i + 6)` at `i = 11` gives `jt = 7`, jumping **past the end of the BPF program** (causing `prctl(PR_SET_SECCOMP)` to fail with `EINVAL`).
  2. **Broken FD Close Range:** `for fd in 3..spec.max_open_fds.clamp(256, 4096)` misses any descriptor `>= 4096` (`RLIMIT_NOFILE` up to 1048576 on modern Linux).
  3. **Tier 1 Exit 126 Masked:** `exec_r` read ignores `EINTR` and leaves Tier 1's `wait_pidfd_and_reap` returning exit code `126`, which `runner.rs` maps to `Ok(RunOutput { exit_code: Some(126) })` rather than `Err(PlatformSetupFailure)`.

### `AUD-05` (Preliminary Finding D — MAJOR): Windows Teardown Verification Is Unsubstantiated
- **Source Location:** [`crates/sandbox/src/platform/windows.rs#L334-L363`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/platform/windows.rs#L334-L363)
- **Observed Code Defects:**
  1. `kill_tree()` ignores `TerminateJobObject` return value (`let _ = TerminateJobObject(...)`), ignores `WaitForSingleObject` timeout (`WAIT_TIMEOUT`), and blindly sets `self.reaped = true`.
  2. `verify_and_cleanup()` ignores `self.kill_tree()` errors and unconditionally returns `surviving_pids: Vec::new()` without calling `QueryInformationJobObject` (`JobObjectBasicProcessIdList` / `JobObjectBasicAccountingInformation`) to prove zero active processes remain in the Job Object.
  3. `si.StartupInfo.hStdInput = INVALID_HANDLE_VALUE` should be a scoped inheritable `NUL` handle via `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` for predictable console CRT behavior.

### `AUD-06` (MAJOR): Capability Probes Overstate Linux Support & Omit Execution Scopes
- **Source Location:** [`crates/sandbox/src/capabilities.rs#L781-L877`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/capabilities.rs#L781-L877)
- **Observed Code Defects:**
  1. `probe_clone_into_cgroup_flag()` checks only `/proc/sys/kernel/osrelease` (`>= 5.7`) rather than empirically probing whether `SYS_clone3` with `CLONE_INTO_CGROUP` is permitted by outer seccomp/container policies.
  2. `probe_linux_cgroup_v2_controllers()` inspects root `/sys/fs/cgroup/cgroup.controllers` instead of delegated parent `test_parent.join("cgroup.controllers")`, and misses `CLONE_NEWPID` in `probe_linux_userns_authorized()`.
  3. `CapabilityEvidenceRecord` in `linux.rs` updates `status` without updating `configuration_result`, `independent_adversarial_verification`, or failure-specific `verification_scope_limit`.

### `AUD-07` (MAJOR / MODERATE): Supervisor Stream & macOS FD Bounds Gaps
- **Source Locations:** [`crates/sandbox/src/supervisor.rs#L121-L260`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/supervisor.rs#L121-L260), [`crates/sandbox/src/platform/macos.rs#L271-L279`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/platform/macos.rs#L271-L279)
- **Observed Code Defects:**
  1. `supervise_generic` (Windows) spawns detached threads with no `terminate_on_overflow` enforcement during execution and abandons blocked reader threads on `recv_timeout`.
  2. `supervise_unix` swallows `child.kill_tree()` error at line 125 (`let _ = child.kill_tree()`).
  3. `macos.rs` `set_cloexec_above_stdio(max_open_fds.clamp(256, 4096))` ignores host `RLIMIT_NOFILE > 4096` (on macOS `RLIMIT_NOFILE` often exceeds 4096).
  4. Cooperative `setsid()` limitation on macOS (`Finding E`): confirmed documented in `capabilities.rs` and stderr warning; must strictly never be claimed as immune process containment.

### `AUD-08` (MODERATE): `ExperimentalCompilationEngine` & Cleanup Reporting Integrity
- **Source Locations:** [`crates/sandbox/src/lib.rs#L242-L266`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/lib.rs#L242-L266), [`crates/sandbox/src/runner.rs#L186-L240`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/runner.rs#L186-L240)
- **Observed Defects:**
  1. `runner.rs` sets `temp_dir_removed` inside `ExecutionEvidenceReport` before `SandboxRunner` drops, so `temp_dir_removed` records `false` while `SandboxRunner` is still alive.
  2. `ExperimentalCompilationEngine::run_experiment()` does not inspect `runner.last_stream_overflow()`, potentially evaluating a truncated overflowing candidate against baseline as normal output if exit code happened to be 0.

---

## 5. Multi-OS Execution Gap (Critical Stop Condition)

- On this macOS host (`x86_64-apple-darwin`), `cargo test --workspace --offline` executed **128 tests** covering core logic, CLI integrity, and macOS cooperative/fail-closed controls.
- `platform/linux.rs`, `syntra_sandbox_supervisor.rs`, and `platform/windows.rs` remain **uncompiled and unexecuted** against native Linux and Windows kernels.
- **`FIND-002` MUST REMAIN OPEN** until every defect (`AUD-01`–`AUD-08`) is specified (`09B`), corrected in source, compiled cleanly on Linux and Windows, and verified with native `ADV-01`–`ADV-10` and `FLT-01`–`FLT-10` test runs.
