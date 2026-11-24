# Aether Engineering Specifications
## Governance, terminology and shared architecture baseline

**Status:** Engineering baseline  
**Applies to:** All ten Aether specifications  
**Normative language:** The terms MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, MAY and OPTIONAL are to be interpreted as requirements keywords.

## 1. Product definition

Aether is a general-purpose systems programming language and an autonomous, whole-program adaptive compilation and execution platform. Its purpose is to improve performance and resource efficiency through the discovery, evaluation and application of optimisation strategies, subject to semantic correctness, safety invariants, portability requirements and developer-defined policy.

Aether is not an AI code generator. Its intelligence engine operates on compiler-produced programme facts, structured strategy descriptions, cost models and experimental evidence. It does not invent or redefine language semantics.

## 2. Architectural layers

The implementation SHALL preserve these logical layers:

1. **Language:** syntax, types, effects, modules, memory and concurrency semantics.
2. **Frontend:** parsing, name resolution, type checking and semantic analysis.
3. **Programme analysis:** typed programme graph, control/data flow, call graph, escape, lifetime and effect analysis.
4. **AIR:** typed, ownership-aware, memory-domain-aware and concurrency-aware intermediate representation.
5. **Verification and optimisation:** deterministic verification, legal transformations and optimisation passes.
6. **Intelligence:** opportunity detection, strategy planning, cost prediction, candidate ranking and explainability.
7. **Experimental compilation:** isolated candidate compilation, validation, benchmarking and evidence collection.
8. **Native backend and runtime:** native code generation, memory domains, scheduler, profiling, adaptation and deoptimisation.
9. **Learning infrastructure:** datasets, models, evaluation, inference and model governance.
10. **Tooling and build system:** CLI, package management, debugger, profiler, build identity and provenance.

The implementation MAY deploy these as separate libraries, processes or services. Logical boundaries and dependency rules are mandatory regardless of deployment topology.

## 3. Authority and trust boundaries

- The language specification is authoritative for observable programme behaviour.
- Deterministic compiler and runtime checks are authoritative for safety and legality.
- Developer policy is authoritative for permitted optimisation and resource use.
- The intelligence engine proposes and ranks strategies; it cannot waive a failed precondition or verifier result.
- ML predictions are estimates, never proofs.
- LLM-generated suggestions are untrusted research input and MUST be converted into structured proposals and independently checked.
- Experimental binaries MUST be isolated from production execution and data.
- A model, profile or optimisation database MUST NOT silently change the strategy of a reproducible or production build.

## 4. Requirement identifiers

Requirements use stable identifiers: `LANG`, `MEM`, `AIR`, `AIE`, `ACE`, `RUN`, `SAFE`, `LEARN`, `BUILD` and `ENG`, followed by a unique number. Implementations, tests and design decisions SHOULD reference these identifiers.

## 5. Shared interfaces and records

The following conceptual records are shared across specifications:

- **ProgrammeIdentity:** source revision, module graph, dependency lock, target, language version and semantic configuration.
- **ProgrammeFacts:** compiler-derived facts with scope, source location, proof status and invalidation dependencies.
- **ProfileSnapshot:** observed runtime measurements, workload identity, hardware identity, sampling method and confidence/uncertainty.
- **Strategy:** immutable, versioned transformation proposal with preconditions, scope, effects, safety obligations, resource bounds and expected benefit.
- **Candidate:** strategy set plus compiler, AIR, target, profile and policy identities.
- **VerificationResult:** verifier identity/version, checked properties, status, diagnostics and evidence references.
- **ExperimentResult:** candidate identity, environment, inputs, correctness result, metrics, uncertainty and reproducibility metadata.
- **ApprovalRecord:** approving principal, policy version, strategy/candidate digest, scope and time.
- **BuildIdentity:** immutable digest over all build-affecting inputs and decisions.
- **ProvenanceRecord:** trace from source and facts through strategies, verification, experiments, approval and emitted artefacts.

Records MUST be versioned, serialisable and validated at trust boundaries. Unknown mandatory fields or incompatible schema versions MUST fail closed.

## 6. Global invariants

1. No optimisation may change observable semantics permitted by the language specification.
2. Safe-language guarantees MUST hold independently of model quality or availability.
3. No strategy may execute unless its preconditions and policy permissions are satisfied.
4. Uncertainty MUST be represented explicitly; observed behaviour MUST NOT be promoted to a static proof.
5. Production execution MUST have a valid baseline or fallback path.
6. All applied non-baseline strategies MUST be traceable to immutable provenance.
7. Compiler, model and runtime failures MUST degrade to a defined safe behaviour, not undefined behaviour.
8. Aether MUST work without an external AI API. Intelligence unavailability MUST NOT prevent deterministic compilation.

## 7. Cross-cutting quality requirements

All components MUST define resource limits, cancellation, timeouts, structured diagnostics, version compatibility and deterministic failure behaviour. Persistent data MUST be integrity-checked. Logs MUST avoid secrets and unnecessary user data. Compiler and runtime interfaces MUST be testable without invoking an LLM or network service.

## 8. Conformance

A component conforms only when its requirements have implementation evidence, automated tests and documented limitations. A passing benchmark does not establish correctness. A passing test suite does not prove universal correctness unless the specific property and proof method support that claim.
