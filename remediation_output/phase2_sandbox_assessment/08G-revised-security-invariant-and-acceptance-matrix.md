# Syntra Phase 2: SECE Sandbox Security Remediation
# Deliverable 08G: Revised Security Invariant & Acceptance Matrix

**Audited Component:** Syntra Autonomous Experimental Compilation Engine (SECE) Sandbox (`crates/sandbox`)  
**Target Milestone:** Phase 2, Stage 6.5 Corrective Package (Resolves Governance Failure in `07C`, `RT-09`, Gates `G1`–`G12`)  
**Date:** 2026-11-24  
**Status:** **REVISED EVIDENCE-BASED MATRIX — SUPERSEDES `07C` §2**

---

## 1. Governance Correction: Honest Evidence Classification

Deliverable `07C` (§2) prematurely declared all 12 security invariants `PASSED` while production code remained frozen and adversarial tests remained unimplemented. This deliverable formally retracts those premature verdicts and classifies every invariant under strict evidence states:

- **`Specified`:** Requirement formulated in threat model / policy docs.
- **`Designed`:** Concrete kernel primitives, state transitions, failure semantics, and test cases formally authored (`08A`–`08F`).
- **`Implemented`:** Production Rust code written and compiled in `crates/sandbox/src/`. *(Currently NOT reached — production freeze active)*
- **`Tested`:** Reproducible adversarial & fault-injection test suite executed with emitted JSON verification records. *(Currently NOT reached)*
- **`IndependentlyVerified`:** External/independent architectural & runtime evidence review accepted. *(Currently NOT reached)*
- **`NotApplicable`:** Inapplicable to current host target or mode (explicitly scoped).

---

## 2. Revised 12 Security Invariants Matrix

| # | Mandatory Security Invariant | Authoritative Design Spec | Planned Verification Tests | Current Evidence State | Required State Before `FIND-002` Closure |
| :- | :--- | :--- | :--- | :---: | :---: |
| **1** | No untrusted execution without verified isolation | `08B` (S7), `08E` (§2.1) | `ADV-09A`, `ADV-02A`, `ADV-03A` | **Designed** | `IndependentlyVerified` |
| **2** | Strict enforcement fails closed | `08B` (§3), `08D` (§2.3, §3.3) | `ADV-09A`, `FLT-04`, `FLT-07` | **Designed** | `IndependentlyVerified` |
| **3** | Guarded execution reports actual enforcement guarantees | `08D` (§2.3, §3.3) | `ADV-09B` | **Designed** | `IndependentlyVerified` |
| **4** | Capability states avoid misleading booleans | `08E` (§2) | `ADV-09A`, `ADV-09B` + schema checks | **Designed** | `IndependentlyVerified` |
| **5** | Linux and macOS/Windows do not claim equivalent guarantees | `08C`, `08D` | `ADV-09A` (cross-platform) | **Designed** | `IndependentlyVerified` |
| **6** | Resource limits cover aggregate process-tree consumption | `08C` (§1–2), `08D` (§2.1) | `ADV-04G`, `ADV-05` | **Designed** | `IndependentlyVerified` |
| **7** | Memory and swap accounting explicitly addressed (incl. direct cgroup birth) | `08C` (§1–2) | `ADV-04A..G` (incl. heavy parent `ADV-04D`) | **Designed** | `IndependentlyVerified` |
| **8** | Process supervision resists PID reuse and lifecycle races | `08B` (§2–3) | `FLT-01`, `FLT-02`, `FLT-03`, `ADV-01A..C` | **Designed** | `IndependentlyVerified` |
| **9** | Standard descriptors & terminal attachment sanitised | `08B` (S6->S7), `08C` (§2) | `ADV-06A..C` | **Designed** | `IndependentlyVerified` |
| **10**| Output collection bounded and deadline-controlled | `08B` (S8->S9), `06A` (§2.1) | `ADV-08A`, `ADV-08B` | **Designed** | `IndependentlyVerified` |
| **11**| Process descendants do not survive sandbox termination | `08B` (§2) | `ADV-01A..C`, `FLT-01..03`, `FLT-10` | **Designed** | `IndependentlyVerified` |
| **12**| Cleanup deterministic, idempotent, and non-silent on error | `08B` (S9->S10), `08F` | `FLT-04`, `FLT-05`, `FLT-06` | **Designed** | `IndependentlyVerified` |

---

## 3. Governance Gates `G1` through `G12` Traceability Matrix

| Gate | Acceptance Condition | Governing Spec | Verification Evidence Required in Stage 7/8 | Current Spec Status |
| :- | :--- | :--- | :--- | :---: |
| **G1** | Supervisor death cannot leave candidate descendants alive | `08B` (§2) | `FLT-01`, `FLT-02`, `FLT-03`, `FLT-10` | Spec Complete (`Designed`) |
| **G2** | Candidate starts inside intended resource-accounting boundary | `08C` (§1) | `ADV-04D` (256 MB heavy Tier 0 parent -> 64 MB cgroup) | Spec Complete (`Designed`) |
| **G3** | Trusted PID 1 manages the PID namespace | `08B` (§1, S2->S5) | `ADV-01B`, `ADV-10` (`/proc` PID 1 != candidate) | Spec Complete (`Designed`) |
| **G4** | Security policy installed before candidate execution | `08B` (S6->S7) | `FLT-07`, `ADV-07` | Spec Complete (`Designed`) |
| **G5** | Filesystem exposure explicitly allowlisted | `08C` (§3) | `ADV-02A`, `ADV-02B`, `FLT-08` | Spec Complete (`Designed`) |
| **G6** | Windows containment not overstated | `08D` (§1–2) | Windows Job tests + Strict mode refusal (`ADV-09A`) | Spec Complete (`Designed`) |
| **G7** | Capability reporting backed by execution-specific evidence | `08E` (§2–4) | JSON `CapabilityEvidenceRecord` emitted per run | Spec Complete (`Designed`) |
| **G8** | Every security invariant has a reproducible adversarial test | `08F` (§2–3) | Co-implemented `ADV-*` and `FLT-*` suites | Spec Complete (`Designed`) |
| **G9** | Infrastructure failures result in deterministic fail-closed behaviour | `08B` (§3), `08F` (§3) | `FLT-01` through `FLT-10` | Spec Complete (`Designed`) |
| **G10**| Existing public API compatibility independently demonstrated | `08G` §4 + Existing Tests | Verified via `crates/sandbox/src/runner.rs`, `crates/cli/src/lib.rs`, and 118-test baseline | **Empirically Verified (API Unchanged)** |
| **G11**| Regression and adversarial test results reproducible | `08E` (§4), `08F` (§1) | Saved logs & JSON records under `remediation_output/evidence/` | Spec Complete (`Designed`) |
| **G12**| Independent reviewer signs final readiness assessment | `08H` | **Pending Independent Review** | **Awaiting Sign-Off** |

---

## 4. Direct Source Audit of Public API Compatibility (`G10`)

Verified directly against current unmodified source files:
- [`crates/sandbox/src/runner.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/runner.rs): `SandboxRunner::{new, with_timeout, with_max_output_bytes, with_env_var, timeout, max_output_bytes, run_id, sandbox_base, candidate_dir, compile_candidate, execute_binary, cleanup}`, `Drop for SandboxRunner`, `RunOutput`, `CompilationOutput`.
- [`crates/sandbox/src/lib.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/src/lib.rs): `ExperimentalCompilationEngine::{run_experiment, approve_candidate}`, `ApprovedStrategy`, `ExperimentReport`, `CandidateExperimentResult`.
- [`crates/cli/src/lib.rs`](file:///Users/Topman/Documents/Projects/aether/crates/cli/src/lib.rs): Consumes `ExperimentalCompilationEngine`, `ApprovedStrategy`, `candidate::StrategyKind`.
- [`crates/sandbox/tests/test_sandbox_isolation.rs`](file:///Users/Topman/Documents/Projects/aether/crates/sandbox/tests/test_sandbox_isolation.rs): Exercises `SandboxRunner`, `RunOutput`, `EquivalenceVerifier`, `Candidate`.

All planned internal refactoring keeps every existing signature, struct field, and `Result<RunOutput, String>` contract intact.
