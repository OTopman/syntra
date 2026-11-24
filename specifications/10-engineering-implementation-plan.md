# 10. Aether Engineering and Implementation Plan
## Repository boundaries, implementation sequence and acceptance gates

**Status:** Normative programme execution plan  
**Planning horizon:** Initial 36-month roadmap; estimates, not delivery guarantees.

## 1. Engineering principles

Aether is a greenfield compiler/runtime project. The team MUST resist building the intelligence and learning layers before it has a correct language, IR, verifier and native backend. A high-performance compiler cannot be established by architecture diagrams, AI agents or benchmark claims. It requires a working semantic core, a trusted compiler pipeline and repeatable evidence.

The implementation MUST prioritise correctness, observability, reproducibility and narrow vertical slices over breadth.

## 2. Repository boundaries

Recommended top-level areas:

- `language/`: specification, grammar, semantics, type system, conformance tests.
- `compiler/frontend/`: lexer, parser, name resolution, type checking, diagnostics.
- `compiler/analysis/`: ownership, effects, control/data flow, programme graph.
- `compiler/air/`: IR structures, verifier, serialisation and pass manager.
- `compiler/optimisation/`: deterministic optimisation passes and analysis invalidation.
- `compiler/backend/`: x86-64 and ARM64 lowering, register allocation, object emission.
- `intelligence/`: coordinator, agents, planner, policies, cost models and reports.
- `experimental/`: candidate generation, sandbox runner and benchmark orchestration.
- `runtime/`: core, allocators, memory domains, GC, scheduler, profiling and adaptation.
- `learning/`: datasets, training, evaluation, inference and model registry.
- `stdlib/`: core, collections, strings, I/O, networking, concurrency, time and system APIs.
- `tooling/`: CLI, build system, package manager, debugger, profiler and benchmark tools.
- `tests/`: conformance, compiler, memory, concurrency, fuzz, integration and regression.
- `benchmarks/`: CPU, memory, networking, concurrency and real-world workloads.
- `research/`: experimental work isolated from production components.
- `docs/`: architecture, specifications, ADRs and developer guides.

## 3. Dependency rules

1. Language specifications do not depend on compiler implementation.
2. Frontend may depend on language definitions, but not on AIE or ML.
3. AIR verifier is deterministic and independent of model inference.
4. Optimisation passes depend on AIR contracts and verifier interfaces.
5. AIE consumes analysis/strategy interfaces; it cannot modify semantic authority.
6. ACE consumes candidates and verification interfaces; it cannot write production artefacts directly.
7. Runtime may consume approved strategy manifests and profiles, not training pipelines.
8. Learning consumes experiment records; production compiler/runtime do not depend on training infrastructure.
9. Tooling orchestrates components through stable interfaces; it must not duplicate business rules.
10. Research prototypes must not become production dependencies without review and acceptance.

CI MUST enforce dependency boundaries and reject circular dependencies.

## 4. Technology guidance

Compiler and runtime implementation SHOULD use a systems language suitable for low-level control, such as C++ or Rust, selected by an explicit engineering decision. Aether's frontend, AIR and core optimiser should be proprietary implementations. Existing assemblers, linkers, object-file libraries, profilers, sanitiser ecosystems and platform interfaces MAY be reused where licences and portability permit.

Python MAY support offline model training and analysis but MUST NOT be a production runtime dependency. Initial targets SHOULD be x86-64 and ARM64. Debug information SHOULD begin with DWARF where applicable.

## 5. Implementation sequence and gates

### Phase 1 — Language and semantic foundations (months 1–3)
Deliver language specification, grammar, type-system design, memory/concurrency semantics, module rules and developer policy format.

**Gate:** normative semantics and conformance tests agreed; unresolved memory/ownership questions block implementation freeze.

### Phase 2 — Frontend and AIR foundation (months 4–6)
Implement lexer/parser, diagnostics, name resolution, type checker, typed AIR, CFG, SSA discipline and verifier.

**Gate:** positive/negative language tests pass; malformed AIR is rejected; no native code generation from unverified AIR.

### Phase 3 — Native compiler baseline (months 7–10)
Implement x86-64 backend, ABI, object generation, linking integration, basic optimisation and C interoperability. ARM64 follows when architecture allows.

**Gate:** correct execution of a defined language subset, differential tests, ABI tests and reproducible baseline builds.

### Phase 4 — Whole-program analysis (months 11–13)
Implement programme graph, call/data-flow summaries, effects, escape/lifetime/allocation analysis and incremental invalidation.

**Gate:** facts carry proof/observation classification; incremental changes invalidate all dependent facts correctly.

### Phase 5 — Memory domains (months 14–17)
Implement ownership enforcement, stack/owned and arena domains, reference counting and initial precise non-moving mark-and-sweep managed domain.

**Gate:** domain transition, root enumeration, reclamation, OOM and concurrency tests pass. No adaptive domain selection before this gate.

### Phase 6 — Intelligence engine (months 18–20)
Implement structured strategies, coordinator, agents as deterministic components, cost models, policy enforcement and explanations.

**Gate:** Observe mode works; approval and policy cannot be bypassed; disabling AIE leaves compilation intact.

### Phase 7 — Experimental compilation (months 21–24)
Implement candidate search, isolated runner, correctness gates, benchmark harness, statistical summaries and Pareto ranking.

**Gate:** sandbox and candidate correctness are validated; recommendations include reproducible evidence; no automatic production promotion.

### Phase 8 — Adaptive runtime (months 25–28)
Implement runtime profiles, scheduler, guarded specialisation, background compilation, code cache and conservative fallback/deoptimisation.

**Gate:** adaptation disabled by default; stress/fault-injection tests prove safe activation, retirement and fallback.

### Phase 9 — Learning system (months 29–32)
Implement dataset pipeline, analytical baselines, supervised models, evaluation, registry, offline inference and governance.

**Gate:** held-out evaluation, model rollback, privacy controls and deterministic fallback work; model absence does not affect correctness.

### Phase 10 — Hardening and ecosystem (months 33–36+)
ARM64 maturity, standard library, package manager, debugger, profiler, security audit, self-hosting research, public benchmark suite and stable language release process.

**Gate:** release criteria below all pass. Self-hosting is a milestone, not a substitute for conformance or reliability.

## 6. Engineering operating model

Every subsystem MUST have an owner, interface contract, threat model, test plan, performance budget and operational diagnostics. Architectural changes require an ADR. Compiler passes require pre/postconditions. Runtime features require failure-mode analysis. Model changes require evaluation reports. Public performance claims require reproducible benchmark artefacts and independent review.

CI SHOULD include formatting, static analysis, unit/integration tests, fuzz smoke tests, sanitiser builds, deterministic build checks and benchmark regression alerts. Performance regression alerts MUST distinguish noisy measurements from repeatable regressions.

## 7. Release criteria

Aether MUST NOT declare a stable systems-language release until:
- Language semantics and ABI are versioned.
- Compiler emits correct native code for the supported subset.
- AIR invariants are enforced.
- Safe ownership and memory-domain transitions are implemented.
- GC root/reclamation behaviour is validated.
- Concurrency semantics and scheduler behaviour are defined.
- FFI contracts are documented.
- Production strategy policies are enforced.
- Candidate experiments are isolated and reproducible.
- Runtime adaptation has correct fallback or remains disabled.
- Build identity and provenance are complete.
- Security review and fuzzing have addressed critical findings.
- Standard tooling and documentation support real development.

## 8. Performance evaluation

Benchmark against optimised C, C++, Rust, Go and Zig using equivalent algorithms, inputs, compiler settings and hardware. Report runtime, tail latency, memory, CPU, compile time and binary size as relevant. Separate baseline compiler performance from gains attributable to AIE/ACE/AAR. Publish workload limitations and confidence intervals. Do not claim general superiority without broad, independently repeatable evidence.

## 9. Principal risks and mitigations

- **Scope explosion:** freeze a minimal language subset and add features only after vertical slices work.
- **Unverifiable optimisation:** restrict transformations to explicit contracts and conservative preconditions.
- **GC complexity:** begin with a precise, non-moving collector in an explicit domain.
- **Search explosion:** prune aggressively and cap every resource.
- **ML overfitting:** use held-out workloads/hardware and deterministic baselines.
- **Runtime instability:** keep adaptation opt-in and preserve baseline code.
- **Ecosystem gap:** prioritise ABI, tooling, package management and debugging before broad marketing.
- **Unsupported performance claims:** require public, reproducible benchmark evidence.

## 10. Project acceptance

A phase is complete only when its deliverables, tests, documentation, threat model, resource limits and gate evidence are reviewed. A feature marked complete in a project tracker without these artefacts is not accepted. Schedule estimates MUST be revised based on measured engineering throughput and defect rates.
