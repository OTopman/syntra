# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 08A: Independent Red-Team Findings and Corrective Disposition

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 Corrective Package (Superseding Conflicting Sections of `06A`–`06D` and `07A`–`07D`)  
**Date:** 2026-11-24  
**Governance Context:** Stage 7 Authorization Currently Withheld (`NO-GO` under Stage 6.5 Independent Review)

---

## 1. Purpose & Governance Classification

Following the Stage 6.5 Independent Governance Review (`NO-GO` decision), this deliverable formally reconciles all findings from the internal red-team review (`07A`) and the independent security architecture review, establishing a single, unambiguous disposition register before Stage 7 can be reconsidered.

In strict alignment with governance requirements, every finding is classified into one of four analytical categories:
- **Valid Finding:** Defect accurately identified during internal or independent review.
- **Incomplete Correction:** Proposed fix that narrows an attack window while leaving exploitable kernel or lifecycle paths open.
- **Incorrect Guarantee:** Architectural statement claiming security assurance beyond what the specified OS mechanism enforces.
- **Governance Failure:** Claiming implementation or verification status without executed code or empirical test evidence.

---

## 2. Reconciliation of `07A` Discrepancies

The independent review identified a numeric inconsistency in [07A-independent-architectural-red-team-review.md](file:///Users/Topman/Documents/Projects/aether/remediation_output/phase2_sandbox_assessment/07A-independent-architectural-red-team-review.md):
- Executive Summary stated: 4 critical defects and 2 operational flaws.
- Body text described: 4 critical defects and 3 subtle race conditions.
- Summary table listed: 6 findings (`RED-01` through `RED-06`).

**Formal Reconciliation:**
- Exact Stage 6.5 internal findings count is **6 primary red-team findings** (`RED-01` through `RED-06`: 3 Critical, 2 Major, 1 Moderate), plus **4 operational risks** (`RISK-01` through `RISK-04`) tracked separately in `07D`.
- The independent governance review subsequently identified **8 detailed architectural challenges (Findings 1–8)** and **10 additional defects (`IND-01` through `IND-10`)**.
- All `RED-*`, `Finding 1–8`, and `IND-*` items are unified in Section 3 below without omission.

---

## 3. Master Red-Team & Independent Findings Disposition Register

| Unified ID | Source Reference | Classification | Severity | Defect / Gap Summary | Corrective Disposition & Target Deliverable |
| :--- | :--- | :--- | :---: | :--- | :--- |
| **RT-01** | `RED-01`, Finding 1, `IND-03` | **Incomplete Correction** + **Incorrect Guarantee** | **CRITICAL** | `07B` proposed Tier 1 Supervisor -> Intermediary -> Candidate PID 1, claiming Tier 1 death sends `SIGKILL` via `PR_SET_PDEATHSIG` and closes `sync_pipe`. In Linux, `PR_SET_PDEATHSIG` tracks creator thread (not Tier 1 across an intermediary), clears on fork/certain credential changes, and pipe EOF fails if any process retains the write FD (`O_CLOEXEC` acts at exec, not fork). | Replaced with 5-layer hierarchy + 3 independent death-containment mechanisms (`cgroup.kill`, `pidfd` on Trusted PID 1, verified parent-death + startup sync barrier). Specified in **08B**. |
| **RT-02** | `RED-02`, Finding 2, `IND-01` | **Incomplete Correction** | **CRITICAL** | `07B` forked candidate before writing PID to `cgroup.procs`. Linux cgroups v2 memory controller does not transfer pre-existing/CoW memory charges across `cgroup.procs` moves; also omits complete resource-exhaustion controls. | Mandated pre-created cgroups v2, sterile Tier 2 creation via `clone3(CLONE_INTO_CGROUP)` (or pre-attached sterile exec fallback policy), and complete multi-resource policy. Specified in **08C**. |
| **RT-03** | `RED-05`, Finding 3, `IND-02` | **Incorrect Guarantee** | **CRITICAL** | `07B` simultaneously designated untrusted candidate as PID 1 and claimed intermediary reaps orphans. Untrusted candidate must never hold PID 1 init privileges/signal immunity or fail reaping. | Mandated **Trusted Namespace Init (PID 1)** separate from **Untrusted Candidate (PID 2+)**; PID 1 runs `PR_SET_CHILD_SUBREAPER` and teardown propagation. Specified in **08B**. |
| **RT-04** | `RED-04`, Finding 4, `IND-05`, `IND-09` | **Incorrect Guarantee** | **BLOCKER / MAJOR** | `07B` reduced capability states to a 5-level ordinal enum and treated narrow container probes (`/etc/shadow` `ENOENT`, 0 netlink routes, `/proc/self/fd`) as proof of complete isolation. | Replaced with multi-field **Capability Evidence Schema** and 6 distinct status classifications (`Unsupported` .. `VerificationFailed`), machine-readable evidence records, and explicit probe scope limits. Specified in **08E**. |
| **RT-05** | `RED-03`, Finding 5, `IND-06` | **Incomplete Correction** | **MAJOR / CRITICAL** | `CREATE_SUSPENDED` + Job Object covers process-creation scheduling window and basic resource limits, but Job Object is **not** a complete Windows security boundary (filesystem, registry, token, network, WMI/COM `Win32_Process.Create` escape paths, nested jobs, `SetInformationJobObject`/`ResumeThread` error checks). | Explicitly separated **Resource Containment** from **Security Containment**; added separate Windows Security Isolation Provider specification and mandatory fail-closed Strict enforcement when absent. Specified in **08D**. |
| **RT-06** | Finding 6, `IND-10` | **Incomplete Correction** | **CRITICAL** | `07B` called an 8-state (`State 0..State 7`) prose description a "7-step state machine" with ambiguous transition ownership, timeouts, failure actions, and cleanup duties; lacked strict trusted-bootstrap vs candidate separation. | Replaced with formal 9-column state-transition specification covering every state, timeout, terminal fail-closed branch, and cleanup owner. Specified in **08B**. |
| **RT-07** | Finding 7, `IND-04` | **Incomplete Correction** | **CRITICAL / MAJOR** | `07D` proposed read-only bind mounts of `/lib`, `/lib64`, `/usr/lib` without a filesystem exposure policy, symlink containment rules, `/proc` / `/sys` / `/dev` / `/run` hardening, writable quota, or inherited FD exclusion. | Replaced with allowlist-only rootfs construction, `MS_PRIVATE`, `pivot_root`, explicit `/proc` & minimal `/dev` policy, writable tmpfs quotas, and static-linking preference. Specified in **08C**. |
| **RT-08** | Finding 8, `IND-07` | **Incomplete Correction** | **CRITICAL / MAJOR** | `ADV-01`–`ADV-10` described broad attack categories without concrete test acceptance criteria, missing sub-variants (shared/file/CoW/swap memory, sockets/dirfds/memfds, concurrent streams), and lacked fault-injection tests. | Expanded into discrete sub-tests (`ADV-01A`..`ADV-10C`) plus dedicated 10-scenario Fault-Injection Suite (`FLT-01`..`FLT-10`) implemented alongside Stage 7 milestones. Specified in **08F**. |
| **RT-09** | `07C` §2, Gate Review | **Governance Failure** | **BLOCKER** | `07C` marked all 12 security invariants `PASSED` based solely on design documentation and 118 baseline regression tests without any implemented sandbox code or adversarial execution. | Retracted premature `PASSED` verdicts; replaced with honest evidence lifecycle states (`Specified`, `Designed`, `Implemented`, `Tested`, `IndependentlyVerified`, `NotApplicable`) tracking everything currently at `Designed`. Specified in **08G** & **08H**. |
| **RT-10** | `RED-06`, Review §2.D | **Valid Finding (Incomplete Test)** | **MODERATE** | macOS `setpgid`/`kill(-pgid)` is defeated by `setsid()`; `ADV-09` only checked Strict refusal without verifying Guarded limitation reporting and Guarded teardown. | Formalized macOS Guarded contract and split `ADV-09A` (Strict refusal) and `ADV-09B` (Guarded warning + teardown verification). Specified in **08D** & **08F**. |
| **RT-11** | Review §3 (`IND-08`) | **Incomplete Specification** | **MAJOR** | Resource policy focused too narrowly on `memory.max` and `pids.max`, leaving CPU, FDs, disk/file size, I/O, network, wall clock, and concurrency under-specified. | Unified 11-dimension Complete Resource-Exhaustion Policy Table. Specified in **08C**. |

---

## 4. Preservation of Central Remediation Register Continuity

All baseline findings in [13-remediation-register.md](file:///Users/Topman/Documents/Projects/aether/remediation_output/13-remediation-register.md) (`FIND-001` through `FIND-013`) remain preserved without modification to the register file itself:
- **`FIND-001` (Package Integrity):** Closed and verified (`phase1-accepted`, 31/31 adversarial tests passing).
- **`FIND-002` (SECE Pseudo-Sandbox):** Remains functionally open under Phase 2 corrective architecture; will **not** be marked closed until Stage 7 implementation, Stage 8 verification, and independent acceptance are completed.
- **`FIND-003` through `FIND-013`:** Continuously protected by the 118-test workspace regression suite.

---

## 5. Supersedence Statement

This package (`08A`–`08H`) supersedes all conflicting architecture, lifecycle, resource accounting, capability modeling, testing, and governance readiness statements in `06A`–`06D` and `07A`–`07D`.

*Deliverable 08A complete. Proceeding to Deliverable 08B.*
