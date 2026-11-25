# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 08C: Linux Cgroup Accounting & Pre-Execution Containment Specification

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 Corrective Package (Resolves Findings 2, 7; `RT-02`, `RT-07`, `RT-11`; `IND-01`, `IND-04`, `IND-08`)  
**Date:** 2026-11-24  
**Status:** **CORRECTED LINUX SPECIFICATION — SUPERSEDES `06B` & `07B` LINUX/FS SECTIONS**

---

## 1. Resolving Pre-Existing Memory Charges (`Finding 2` / `IND-01`)

### 1.1 Kernel Semantics of Process Migration (`cgroup.procs`)
Under Linux cgroups v2 memory controller (`Documentation/admin-guide/cgroup-v2.rst`):
- A child created via `fork()` / `clone()` initially inherits the parent's cgroup and references the parent's Copy-on-Write (CoW) anonymous pages, page cache charges, and kernel allocations.
- Moving an already-forked process by writing its PID to `/sys/fs/cgroup/.../cgroup.procs` **does not migrate pre-existing memory charges** (anon CoW pages or cached file charges) from the parent's cgroup into the destination cgroup unless explicit charge migration applies (and cgroups v2 intentionally omits v1's charge-move behavior for stability/performance reasons).
- If a memory-heavy Tier 0 compiler process (e.g., holding hundreds of megabytes of AST/AIR/Cranelift structures) forks a sandbox process and only later migrates it, inherited CoW and page-cache pressure remain misattributed to the compiler's own host cgroup.

### 1.2 Mandatory Direct Cgroup Placement Architecture
To eliminate misattributed initial memory charges:
1. **Tier 1 Sterile Supervisor Boundary:** Tier 0 (`syn` compiler) **never** forks the sandbox container directly. Tier 0 spawns a sterile, minimal Tier 1 supervisor process via `execve()` (dropping 100% of Tier 0's compiler heap and CoW address space).
2. **Pre-Spawn Cgroup Creation & Limit Configuration:** Before spawning Trusted PID 1, Tier 1 creates `/sys/fs/cgroup/syntra_sandbox/<run_id>`, configures all resource ceilings (`memory.max`, `memory.swap.max`, `memory.high`, `memory.oom.group`, `pids.max`, `cpu.max`, `io.max` if available), and opens `cgroup_fd = open(..., O_RDONLY | O_DIRECTORY | O_CLOEXEC)`.
3. **Primary Kernel Placement (`clone3` + `CLONE_INTO_CGROUP`):** On Linux 5.7+, Tier 1 spawns Trusted PID 1 directly into `cgroup_fd` via `clone3(CLONE_INTO_CGROUP | CLONE_PIDFD | CLONE_NEWUSER | CLONE_NEWNS | CLONE_NEWNET | CLONE_NEWPID)`. Trusted PID 1 and all future PID 2+ descendants are born inside the target cgroup at tick 0 with zero intermediate cgroup window.
4. **Sterile Fallback When `CLONE_INTO_CGROUP` Returns `ENOSYS` (Kernels 5.3–5.6 or Filtered Docker):**
   - If `clone3(CLONE_INTO_CGROUP)` is blocked or unsupported, Tier 1 must **never** accept CoW contamination from Tier 0.
   - Instead, Tier 1 (already cleanly `execve`'d) writes *itself* (or a fresh sterile exec'd helper) into `<run_id>/cgroup.procs`, executes `execve()` to clear all prior VM mappings/charges, and only then spawns Trusted PID 1 (which inherits the destination cgroup directly at `clone()` time) — OR, if cgroup write permissions deny moving Tier 1 before exec, Strict mode fails closed (`Unavailable`/`PartiallyEnforced`).
5. **Mandatory Verification Test (`Finding 2`):** A dedicated test spawns Tier 0 with $\ge 256\text{ MB}$ dirty anonymous allocations, launches a strict sandbox configured with `memory.max = 64 MB`, and confirms both that sandbox allocation pressure hits the `64 MB` sandbox cgroup boundary and that Tier 0 remains undisturbed.

---

## 2. Complete Resource-Exhaustion Policy (`IND-08`)

`memory.max` alone is an incomplete resource policy. The Linux Tier 1 & Tier 2 bootstrap layers must configure and document all 11 resource classes below:

| # | Resource Class | Enforcement Mechanism (Linux) | Required Guarantee | Accounting / Failure Handling |
| :- | :--- | :--- | :--- | :--- |
| **1** | **Resident & Charged Memory** | Cgroups v2 `memory.max` (+ `memory.high` at 85%) | Hard ceiling on anon, page cache, kernel slab/socket/dentry charges for entire sandbox tree | Check `memory.events` (`oom`, `oom_kill`, `oom_group_kill`); never substitute `memory.oom.group=1` for explicit `cgroup.kill` teardown |
| **2** | **Swap Usage** | Cgroups v2 `memory.swap.max = 0` | Zero swap allocation permitted; prevents swapping past `memory.max` to host disk | If `memory.swap.max` file does not exist (host kernel swap disabled), verify `/proc/swaps` empty or mark explicit note |
| **3** | **Process / Thread Count** | Cgroups v2 `pids.max` (default: 32; configurable) | Aggregate cap across Trusted PID 1, Candidate PID 2+, and all threads/forks | `fork()` / `clone()` returns `EAGAIN` past cap; tracked in `pids.events` (`max`) |
| **4** | **CPU Time & Bandwidth** | Cgroups v2 `cpu.max` (e.g. `100000 100000` for 1 core quota) + `RLIMIT_CPU` | Bounds aggregate multi-core CPU burn across all processes + threads | Throttled by CFS bandwidth; `cpu.stat` (`nr_throttled`) recorded |
| **5** | **File Descriptors** | `RLIMIT_NOFILE` per process + `close_range(3, ~0U, 0)` + Seccomp open/socket caps | Caps per-process FDs (e.g. 64); zero leaked host FDs > 2 cross `execve` | `EMFILE` when exceeded; audited via isolated `/proc/self/fd` |
| **6** | **Disk / Storage Quota** | Container root `tmpfs` mounted with `size=<bytes>,nr_inodes=<count>` | Bounds total writable filesystem bytes & inodes strictly in container tmpfs | `ENOSPC` when candidate attempts disk-fill attack; host disk untouched |
| **7** | **Individual File Size** | `RLIMIT_FSIZE` on Tier 2 before `execve` | Bounds single-file write size even within tmpfs quota | `EFBIG` / `SIGXFSZ` on runaway single file writes |
| **8** | **Block / Storage I/O** | Tmpfs-backed storage + optional Cgroups v2 `io.max` (if underlying blkio exists) | Zero direct write access to host block devices; `/dev` has no disk nodes | Host I/O bandwidth protected from runaway sync/write floods |
| **9** | **Network Bandwidth & Sockets** | Unshared `CLONE_NEWNET` (`lo` DOWN, 0 routes) + Seccomp `socket()` blocking (`AF_INET`/`AF_INET6`/`AF_NETLINK`/`AF_PACKET`) | Zero external or host loopback egress/ingress | Connection attempts fail before leaving kernel stack |
| **10** | **Output Stream Volume** | Independent `stdout_limit_bytes` and `stderr_limit_bytes` with `O_NONBLOCK` `poll()` | Independent bounds regardless of kernel 64 KB pipe buffer capacity | `overflow_detected` triggers `cgroup.kill` teardown and `StreamOverflow` |
| **11** | **Wall-Clock Time & Concurrency** | Tier 1 monotonic deadline + 500 ms drain window + per-host semaphore/slot cap | Hard execution deadline + bounded concurrent sandbox count | `TimeoutExceeded` triggers `cgroup.kill`; bounded active cgroups |

---

## 3. Explicit Allowlist Filesystem Exposure Policy (`Finding 7` / `IND-04`)

Blanket read-only bind mounts of host directories (e.g. `/usr/lib`, `/lib`) expose unintended host packages, metadata, or readable secrets. The Linux provider must construct an **allowlist-driven minimal rootfs**:

### 3.1 Root Construction Rules
1. **Private Mount Propagation:** Trusted PID 1 executes `mount(None, "/", None, MS_REC | MS_PRIVATE, None)` immediately after entering `CLONE_NEWNS` so no mount/unmount event propagates to or from the host.
2. **Empty Quota-Bounded Root:** Mounts a fresh `tmpfs` at `<sandbox_root>/rootfs` with `size=32M,nr_inodes=2048,mode=0755`.
3. **Directory Allowlist Only:** Only explicitly required directories are created inside `<sandbox_root>/rootfs`:
   - `/bin` (only if explicitly needed; omitted by default for native ELF candidate execution)
   - `/candidate` (read-only bind or copy of the single compiled candidate ELF binary, `0555`)
   - `/tmp` (separate quota-bounded `tmpfs`, `nodev,nosuid,noexec,size=16M,nr_inodes=512`)
   - `/proc` (fresh container-private `proc` mount, `nodev,nosuid,noexec`, with `/proc/sys` and `/proc/sysrq-trigger` over-mounted read-only or empty tmpfs)
   - `/dev` (fresh minimal `tmpfs` with `mode=0755`, containing **only** safe virtual devices or bind-mounted individual character files: `/dev/null`, `/dev/zero`, `/dev/urandom` — never `/dev/mem`, `/dev/kmem`, `/dev/tty`, raw disks, or host sockets)
4. **Runtime Library Policy (Closing Host `/usr/lib` Exposure):**
   - **Preferred Mode (Zero Host Lib Exposure):** Link SECE experimental native candidate binaries **statically** (e.g. static musl libc / static `syntra_rt.a`) where feasible, requiring **zero** host `/lib`, `/lib64`, or `/usr/lib` mounts.
   - **Dynamic Binary Mode (Strict File-Level Allowlist):** When a dynamically linked binary is executed (e.g. host glibc binary in tests), Trusted PID 1 parses the binary's `ELF PT_INTERP` and `DT_NEEDED` set (or uses an explicit immutable allowlist of exact files: e.g. `/lib64/ld-linux-x86-64.so.2`, `libc.so.6`, `libm.so.6`, `libpthread.so.0`, `libdl.so.2`, `libgcc_s.so.1`) and bind-mounts **only those resolved regular files** (canonicalized with symlinks resolved and contained, never whole `/usr/lib` trees!) read-only (`MS_BIND | MS_RDONLY | MS_NODEV | MS_NOSUID`).
5. **`pivot_root` + Host Root Unmount:**
   - Calls `pivot_root(new_root, put_old)`, `chdir("/")`, `umount2(put_old, MNT_DETACH)`, and removes the temporary pivot directory.
   - Re-mounts root read-only (`MS_REMOUNT | MS_RDONLY`) after setup while leaving `/tmp` as the only bounded writable region (`noexec,nosuid,nodev`).
6. **Zero Host `/sys`, `/run`, `/home`, `/etc` Exposure:**
   - `/sys`, `/run`, `/var`, `/home`, `/root`, and host `/etc` are **absent** (never mounted). If `/etc/ld.so.cache` is strictly required for dynamic lookup, an empty placeholder or explicit `LD_LIBRARY_PATH=/lib:/lib64` pointing exclusively to allowlisted mounts is used instead of exposing host `/etc`.
7. **Symlink & Path-Traversal Defense:**
   - Every candidate path and allowlisted runtime file is opened with `O_NOFOLLOW` / `openat2(..., RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH)` (where supported) or step-by-step directory FD traversal before mounting, preventing symlink-based mount escapes.

---

## 4. Seccomp-BPF Policy (`ADV-07` & Syscall Boundary)

Installed by **Tier 2 Trusted Bootstrap** immediately after all mounts/FDs are finalized and immediately before `execve(candidate)`:
1. Sets `prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)` (mandatory precondition for unprivileged seccomp).
2. Loads BPF filter on target architecture (`AUDIT_ARCH_X86_64` or `AUDIT_ARCH_AARCH64`):
   - **Blocked with `EPERM`:**
     - Async I/O bypasses: `io_uring_setup`, `io_uring_enter`, `io_uring_register`
     - Debugging / memory tampering / kernel loading: `ptrace`, `process_vm_readv`, `process_vm_writev`, `bpf`, `kexec_load`, `kexec_file_load`, `perf_event_open`, `userfaultfd`
     - Namespace / mount escape calls: `unshare`, `setns`, `mount`, `umount2`, `pivot_root`, `chroot`, `swapon`, `swapoff`
     - Privileged / raw socket calls: `socket` filtered to reject `AF_PACKET`, `AF_NETLINK`, `AF_INET`, `AF_INET6` in Strict mode (blocking protocol family creation at syscall boundary in addition to `CLONE_NEWNET`)
     - Keyring / module syscalls: `add_key`, `keyctl`, `request_key`, `init_module`, `finit_module`
3. **Kernel Version Handling (`ADV-07`):** On older kernels where `io_uring_*` syscalls do not exist, invoking those syscall numbers returns `ENOSYS` natively or `EPERM` from Seccomp-BPF; both verify that `io_uring` cannot be initialized (`syscall < 0` and neither ring FD nor sq/cq maps exist).

---

## 5. Deterministic OOM Telemetry & Teardown

- `memory.oom.group = 1` ensures the kernel OOM killer kills the entire cgroup together if memory pressure hits `memory.max`.
- However, `memory.oom.group` is **never treated as the teardown mechanism**: Tier 1 always executes explicit `cgroup.kill` (or `cgroup.freeze` + iterative `SIGKILL` loop) on any deadline, overflow, failure, or normal exit.
- After teardown completes, Tier 1 reads `memory.events`:
  - `oom`: cgroup hit `memory.max`
  - `oom_kill`: kernel killed at least one process
  - `oom_group_kill`: kernel killed the cgroup process group
- Memory exhaustion is reported when `(oom > 0 || oom_kill > 0 || oom_group_kill > 0)` AND confirmed alongside cgroup peak stats (`memory.peak` / `memory.current`).
