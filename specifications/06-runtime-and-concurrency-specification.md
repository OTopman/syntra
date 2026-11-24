# 6. Aether Runtime and Concurrency Specification
## Scheduler, runtime adaptation and deoptimisation

**Status:** Normative runtime contract

## 1. Objectives

The Aether Adaptive Runtime (AAR) executes native code, manages runtime resources and may activate previously validated specialisations. Runtime adaptation is optional and policy-controlled. Baseline native execution MUST remain available.

## 2. Runtime components

- Runtime Core and initialisation.
- Memory Domain Manager.
- Allocators and managed-heap/GC integration.
- Task Scheduler and work-stealing queues where supported.
- Synchronisation, atomics, channels and I/O integration.
- Profile Collector.
- Adaptation Coordinator.
- Guard Manager.
- Code Cache.
- Deoptimisation Manager.
- Resource Monitor.
- Runtime Policy Manager.

## 3. Concurrency model

The language specification defines concurrency semantics; the runtime implements them. The initial scheduler SHOULD support structured task lifetimes, bounded worker pools, cooperative cancellation and work stealing. Thread creation MUST be bounded by runtime policy.

**RUN-001** Tasks MUST have defined ownership, completion, error and cancellation semantics.

**RUN-002** A task MUST NOT outlive resources it is permitted to access.

**RUN-003** Shared mutable state MUST be accessed through language-approved synchronisation or atomic mechanisms.

**RUN-004** Scheduler queues and wake-up mechanisms MUST avoid lost wake-ups and preserve required ordering.

**RUN-005** Runtime shutdown MUST define treatment of outstanding tasks, I/O and managed resources.

The runtime MUST provide back-pressure or bounded queues for workload paths that could otherwise grow without limit.

## 4. Profiling

Profile collection MAY measure hot functions, branch behaviour, allocation rates, object lifetimes, lock contention, task scheduling, cache/NUMA signals and latency distributions.

**RUN-010** Profiles MUST be associated with workload, build and hardware identities.

**RUN-011** Sampling MUST have configurable overhead limits and be disableable.

**RUN-012** Runtime observations MUST be labelled as observations, not static proofs.

**RUN-013** Profile buffers MUST be bounded and must not expose sensitive payload contents by default.

## 5. Adaptation loop

The runtime loop is: baseline execution; bounded profile collection; opportunity detection; candidate specialisation request; verification and policy checks; background compilation; guarded activation; monitoring; deoptimisation or retirement if assumptions fail.

The initial release SHOULD limit runtime adaptation to background compilation of a targeted function or region and controlled code replacement at safe activation points.

**RUN-020** Runtime adaptation MUST be disabled unless explicitly permitted by developer policy.

**RUN-021** Runtime adaptation MUST NOT mutate language-visible state while compiling a replacement.

**RUN-022** A specialisation MUST have guards for every dynamic assumption on which correctness depends.

**RUN-023** Guard failure MUST transfer execution to a semantically equivalent fallback.

**RUN-024** Code replacement MUST be thread-safe and MUST preserve active stack/frame validity.

**RUN-025** Specialised code MUST be retired only when no thread can execute or return into it.

## 6. Deoptimisation

The deoptimisation manager MUST support:
- Guard-failure detection.
- Mapping specialised machine state to a valid baseline/interpreter-independent continuation state.
- Reconstruction of required locals, stack values and ownership state.
- Safe transfer to fallback code.
- Retirement of invalid specialisations.
- Recording the reason and provenance.

If state reconstruction cannot be proven correct, the specialisation MUST NOT be activated. The initial runtime MAY avoid transformations that require complex deoptimisation and use safe-point replacement instead.

## 7. Code cache

The code cache MUST be bounded, keyed by complete specialisation identity and protected against stale or incompatible entries. Cache entries MUST include target, compiler, AIR, strategy, guard and policy identities. Eviction MUST not invalidate code currently in use.

## 8. Resource and failure policy

The runtime MUST bound profiling overhead, background compiler CPU, code-cache size, adaptation frequency and memory usage. On resource pressure, it SHOULD stop adaptation before degrading application correctness or availability. If profiling, model inference or background compilation fails, baseline execution continues.

A runtime invariant violation, corrupted code-cache entry or failed ownership reconstruction MUST trigger a fail-stop or documented process-isolation response; continuing with untrusted state is prohibited.

## 9. Security

JIT/experimental code generation, if enabled, MUST use W^X or equivalent executable-memory protections, code signing/integrity checks where applicable, strict target validation and least privilege. No untrusted model output may be directly executed. Runtime telemetry MUST be access-controlled and redacted.

## 10. Testing and acceptance

Test scheduler fairness and starvation, cancellation, back-pressure, race detection, profile overhead, guard correctness, activation under concurrent load, deoptimisation state reconstruction, code-cache eviction, shutdown and fallback under compiler/model failure.

**Acceptance gate:** Runtime adaptation remains off by default until fallback, guards, activation, retirement and deoptimisation have passed stress and fault-injection tests. Baseline native execution must not depend on adaptive services.
