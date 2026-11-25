# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 08E: Capability Evidence & Verification Specification

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 Corrective Package (Resolves Finding 4; `RT-04`; `IND-05`, `IND-09`)  
**Date:** 2026-11-24  
**Status:** **CORRECTED CAPABILITY SPECIFICATION — SUPERSEDES `06B` §2–3 & `07B` §5**

---

## 1. Purpose & Rejection of Simplistic Probe Claims

Deliverable `07B` (§5) modeled capability maturity as a single linear 5-step enum and treated isolated container checks (e.g. `stat("/etc/shadow") == ENOENT`, netlink returning 0 routes, `getpid() == 1`, `/proc/self/fd` showing `[0, 1, 2]`) as definitive verification of complete isolation.

As established in the independent security review (Finding 4):
1. A linear ordinal enum cannot distinguish between *untested configuration*, *partial enforcement*, and *failed verification*.
2. Narrow point checks confirm local preconditions; they **do not** prove absence of symlinks, hidden mounts, raw socket capabilities, memory charge leaks, or descendant escapes.
3. Checks that inspect `/proc` depend directly on the `/proc` mount configuration itself and therefore cannot independently validate `/proc` isolation without external host/kernel verification.

---

## 2. Revised Capability Classification & Evidence Schema

### 2.1 Non-Linear Capability Status Classification
Every capability in an execution report carries one of **six explicit statuses**:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityVerificationStatus {
    /// OS/architecture fundamentally lacks the primitive (e.g. Linux namespaces on macOS)
    Unsupported,
    /// Host OS supports the primitive, but current user/runtime lacks authorization or kernel config
    Unavailable,
    /// Cooperative or incomplete boundary (e.g. macOS kill(-pgid), Windows Job without AppContainer)
    PartiallyEnforced,
    /// Isolation directives issued for this execution, but kernel & adversarial proof not yet confirmed
    ConfiguredUnverified,
    /// Active kernel enforcement observed AND execution-scoped verification checks passed (scoped to tested vector)
    EnforcedAndVerified,
    /// Configuration or runtime verification failed (mandatory trigger for Fail-Closed teardown)
    VerificationFailed,
}
```

**Strict Mode Acceptance Rule:**
`EnforcementMode::Strict` permits candidate execution **only** if pre-execution configuration succeeds for every mandatory capability **and** no capability is `Unsupported`, `Unavailable`, `PartiallyEnforced`, or `VerificationFailed`. At completion, an execution is marked `StrictVerified` only when every mandatory capability finishes in `EnforcedAndVerified`.

### 2.2 Mandatory Per-Capability Evidence Record
Every capability evaluation produces a structured, auditable record rather than a bare enum variant:

```rust
pub struct CapabilityEvidenceRecord {
    pub capability_id: String,                         // e.g. "linux.memory_swap_containment"
    pub status: CapabilityVerificationStatus,
    pub required_enforcement_mechanism: String,        // e.g. "Cgroups v2 memory.max + memory.swap.max=0 + CLONE_INTO_CGROUP"
    pub host_prerequisites: Vec<String>,               // e.g. ["cgroup2_unified_mount", "memory_controller_delegated"]
    pub configuration_result: String,                  // Exact syscalls/files written and return codes
    pub kernel_observed_enforcement_evidence: String,  // e.g. "/proc/<pid>/cgroup path + mountinfo + netlink dump"
    pub independent_adversarial_verification: String,  // Linked ADV/FLT test IDs and scoped assertion summary
    pub verification_scope_limit: String,              // Explicit statement of what the probe proves vs does NOT prove
    pub failure_reason: Option<String>,
    pub platform_and_kernel_version: String,           // e.g. "linux 6.8.0-x86_64"
    pub evidence_timestamp_unix_ms: u128,
    pub execution_id: String,
}
```

---

## 3. Scope & Limitations of Runtime Probes vs. Kernel/Adversarial Proof

This table defines **what each check establishes** and **what it explicitly does NOT prove**:

| Capability ID | Runtime Pre-Exec / Kernel Check | What It Actually Establishes | What It Explicitly Does NOT Prove | Complementary Proof Required (`08F`) |
| :--- | :--- | :--- | :--- | :--- |
| **`fs.mount_isolation`** | Verify `pivot_root` succeeded via host-visible `/proc/<p1>/mountinfo` (checking only allowlisted tmpfs/ro-file/proc/dev mounts exist and propagation flag is private) | That the container mount table contains only allowlisted mounts at launch | Does not prove symlink/path-traversal resistance or absence of inherited host dirfds | `ADV-02` (secret paths), `ADV-10` (`/proc/sys` ro), `ADV-06B` (inherited dirfd `openat`), `FLT-08` (symlink traversal) |
| **`net.egress_isolation`** | Netlink `RTM_GETLINK` & `RTM_GETROUTE` inside netns + Seccomp `socket(AF_INET/AF_INET6/AF_PACKET/AF_NETLINK)` block | That netns has only `lo` (DOWN), 0 routes, and socket filter loaded at launch | Does not prove inherited host sockets weren't passed across `execve` | `ADV-03` (UDP/TCP/loopback connect + host listener 0-byte check), `ADV-06A` (inherited host socket) |
| **`proc.tree_containment`** | Tier 1 holds `pidfd_pid1` & `cgroup_fd`; `/proc/<p1>/status` confirms PID 1 in isolated PID ns (`NSpid` length 2), candidate PID $\ge 2$ | That container has isolated PID tree with Trusted PID 1 inside target cgroup | Does not prove descendants can't survive Tier 1 crash or `setsid()` double-fork | `ADV-01A..F` (double-fork, `setsid`, Tier 0/1 crash), `FLT-01`, `FLT-02`, `FLT-03` |
| **`mem.cgroup_accounting`** | `clone3(CLONE_INTO_CGROUP)` exit code 0 + verify `cgroup.procs` & `memory.max` / `memory.swap.max=0` from Tier 1 | That PID 1 was born inside target cgroup with configured ceilings | Does not prove heavy Tier 0 parent didn't leak charges or file/shm/swap bypass | `ADV-04A..G` (anon, file mmap, shm, heavy-parent CoW, swap, concurrent) |
| **`fd.sanitization`** | Before `execve`, Tier 2 checks open FD set strictly `{0, 1, 2}` and FD 0 points to `/dev/null` + `isatty(0) == 0` | That bootstrap purged inherited host FDs before `execve` | Does not guarantee Tier 2 itself didn't fail open if `close_range` errored (unless checked!) | `ADV-06A..F` (sockets, dirfds, `memfd`, pipes, duplicated bootstrap handles) |

---

## 4. Machine-Readable Verification Record (`IND-09`)

Every execution of the adversarial/fault-injection test suite (and optionally `--emit-evidence` on sandbox runs) writes a deterministic, reproducible evidence JSON artifact under `remediation_output/evidence/runs/<execution_id>.json`:

```json
{
  "schema_version": "1.0.0",
  "execution_id": "sece_adv_04d_20261124_001",
  "git_commit": "<HEAD_SHA>",
  "platform_os": "linux",
  "kernel_release": "6.8.x",
  "enforcement_mode": "Strict",
  "runtime_config": {
    "memory_max_bytes": 67108864,
    "swap_max_bytes": 0,
    "pids_max": 16,
    "stdout_limit_bytes": 65536
  },
  "capabilities": [
    {
      "capability_id": "mem.cgroup_accounting",
      "status": "EnforcedAndVerified",
      "required_enforcement_mechanism": "Cgroups v2 CLONE_INTO_CGROUP + memory.max + memory.swap.max=0",
      "host_prerequisites": ["cgroup2_unified", "memory_controller"],
      "configuration_result": "cgroup created; limits written; clone3 CLONE_INTO_CGROUP ok",
      "kernel_observed_enforcement_evidence": "memory.events: oom=1, oom_kill=1, oom_group_kill=1",
      "independent_adversarial_verification": "ADV-04D: heavy 256MB parent CoW stress passed",
      "verification_scope_limit": "Verified for anon/CoW/shm under tested workload up to 64MB ceiling",
      "failure_reason": null,
      "platform_and_kernel_version": "linux 6.8.x",
      "evidence_timestamp_unix_ms": 1795546675000,
      "execution_id": "sece_adv_04d_20261124_001"
    }
  ],
  "exit_status": "Killed(OOM)",
  "post_run_cleanup": {
    "cgroup_procs_empty": true,
    "cgroup_removed": true,
    "temp_dir_removed": true,
    "surviving_pids": []
  }
}
```
