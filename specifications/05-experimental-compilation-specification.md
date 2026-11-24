# 5. Aether Experimental Compilation Specification
## Search, candidate evaluation and benchmarking

**Status:** Normative candidate experimentation contract

## 1. Purpose

The Aether Experimental Compilation Engine (ACE) creates isolated alternative compilation plans and evaluates them against a baseline. It is the evidence-producing mechanism for adaptive optimisation. It MUST NOT experiment directly on production state or silently replace production binaries.

## 2. Candidate model

A Candidate MUST identify:
- ProgrammeIdentity and source/dependency digests.
- AIR schema and digest.
- Compiler/backend versions.
- Target architecture and enabled features.
- Strategy IDs and versions.
- Model/cost-model versions used to select it.
- ProfileSnapshot identity, if any.
- Developer policy digest.
- Build flags and environment constraints.

Candidate identity MUST be immutable. A change to any build-affecting input creates a new candidate.

## 3. Search dimensions and controls

Candidate dimensions MAY include memory strategy, task/parallel execution, data layout, dispatch, inlining, vectorisation and target-specific code generation.

Search SHOULD combine deterministic pruning, analytical cost models, profile-guided information, Bayesian optimisation, multi-fidelity evaluation and bounded evolutionary search where justified.

**ACE-001** The engine MUST prune candidates that violate language semantics, verifier invariants, target constraints or developer policy before benchmarking.

**ACE-002** The engine MUST cap candidate count, search depth, compile CPU, memory, disk and wall-clock time.

**ACE-003** Candidate generation MUST be reproducible under pinned inputs and algorithm versions.

**ACE-004** The baseline MUST be retained as a comparison point and fallback.

## 4. Candidate lifecycle

1. Opportunity identification.
2. Strategy composition.
3. Preconditions and compatibility validation.
4. AIR transformation and verification.
5. Isolated native compilation.
6. Correctness tests and workload validation.
7. Benchmark execution.
8. Statistical analysis and uncertainty estimation.
9. Multi-objective ranking.
10. Recommendation and approval.
11. Promotion only through an authorised build/runtime path.

A candidate failing verification or correctness testing MUST NOT proceed to performance ranking as a viable candidate.

## 5. Experiment isolation

The experiment runner MUST execute candidates in a sandbox with explicit CPU, memory, filesystem, network, process, syscall and time limits appropriate to the host platform. Default network access SHOULD be disabled. Inputs MUST be synthetic, sanitised or explicitly approved. Experiments MUST NOT receive production credentials or unrestricted production data.

The runner MUST capture environment identity: CPU model/features, core topology, memory, OS/kernel, runtime versions, power/performance settings where available, compiler flags and benchmark harness version.

## 6. Benchmark methodology

Metrics SHOULD include execution time, throughput, p50/p95/p99 latency, peak/resident memory, allocation rate, CPU utilisation, contention, energy where reliably measurable, binary size and compile time.

**ACE-010** Benchmarks MUST use equivalent workloads, inputs and correctness expectations across candidates.

**ACE-011** The engine MUST detect or report noisy measurements, thermal/frequency variation, background load and insufficient sample sizes where possible.

**ACE-012** Results MUST include sample count, summary statistic, dispersion/uncertainty and environment metadata.

**ACE-013** The engine MUST avoid claiming improvement where confidence or repeatability is insufficient.

**ACE-014** The engine MUST preserve a Pareto frontier or equivalent multi-objective representation. It MUST NOT collapse trade-offs into one universal score without developer-defined priorities.

## 7. Evaluation and promotion

Candidate comparison MUST be workload-specific. Improvements in mean runtime MUST NOT conceal unacceptable tail latency, memory growth, compile-time cost or binary-size regressions. A promotion policy MUST specify acceptable regressions and minimum evidence.

Promotion requires:
- Valid verifier results.
- Correctness suite pass.
- Benchmark evidence and uncertainty.
- Policy authorisation.
- Immutable approval/provenance record.
- A rollback/fallback strategy.

## 8. Result data model

ExperimentResult MUST include candidate digest, baseline digest, workload identity, environment, test suite identity, correctness status, metric samples/summaries, uncertainty, resource consumption, runner logs, termination reason and artifact digests. Results MUST be append-only or tamper-evident.

## 9. Failure handling and security

Timeout, OOM, crash, sandbox escape alert, invalid output, measurement corruption or incomplete test execution MUST mark the experiment failed or inconclusive. A failed run MUST NOT be recorded as a performance win. Runner compromise MUST invalidate affected results and trigger isolation review.

## 10. Testing and acceptance

Test sandbox boundaries, resource limits, cancellation, deterministic candidate IDs, baseline retention, benchmark repeatability, statistical reporting, malformed candidate rejection, correctness gating, result integrity and rollback.

**Acceptance gate:** ACE cannot promote a strategy until isolation, correctness gating, reproducibility and evidence recording are operational. Early versions SHOULD benchmark offline and produce recommendations only.
