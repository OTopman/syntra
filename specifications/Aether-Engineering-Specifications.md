# Aether Engineering Specifications
**Version:** 0.1 architecture baseline  
**Date:** 21 November 2026  
**Status:** Draft for engineering review
> This specification set is internally consistent with the agreed Aether architecture. It is a normative design baseline, not evidence that the compiler/runtime has been implemented or formally verified.

---

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

---

# 1. Aether Master Language Specification
## Language semantics, syntax, type system and modules

**Status:** Normative language design baseline  
**Scope:** Source-language contract for all conforming Aether implementations.

## 1. Goals and non-goals

Aether SHALL provide a general-purpose, statically typed systems language with predictable native execution, explicit effects, safe memory semantics and first-class support for multiple memory domains. It SHALL support systems programming, services, concurrent applications and performance-sensitive workloads.

Aether SHALL NOT make runtime AI availability a language requirement. The language SHALL NOT promise that every programme can be statically proven safe or that one memory strategy is optimal for all workloads.

## 2. Core semantic model

**LANG-001** Aether MUST define observable behaviour independently of compiler implementation, target architecture and optimisation level.

**LANG-002** The specification MUST define evaluation order, integer overflow behaviour, floating-point behaviour, mutation visibility, error propagation, destruction timing where observable, and concurrency memory ordering.

**LANG-003** Safe Aether code MUST have defined behaviour. Undefined behaviour MUST be restricted to explicitly marked unsafe operations and documented foreign-interface obligations. A compiler MUST NOT silently treat an ordinary failed check as permission for undefined behaviour.

**LANG-004** The language MUST distinguish compile-time constants, runtime values and compile-time evaluation. Compile-time execution MUST be resource-bounded and MUST NOT gain unrestricted filesystem, network or host-process access.

**LANG-005** Observable side effects MUST be represented in the language's effect model sufficiently for the compiler to preserve ordering and avoid illegal elimination or duplication.

The initial semantic baseline SHOULD use deterministic left-to-right evaluation for expressions with side effects, explicit integer overflow rules, explicit checked/unchecked arithmetic operations and specified floating-point modes. The exact choices MUST be frozen in the language specification before a stable ABI is declared.

## 3. Syntax and grammar governance

The language SHALL have a formally maintained grammar, lexical rules, reserved-word list, operator precedence table and versioning policy.

**LANG-010** The grammar MUST be unambiguous or define deterministic disambiguation rules.

**LANG-011** Parsing MUST produce source spans and stable syntax-node identities sufficient for diagnostics, incremental analysis and tooling.

**LANG-012** Syntax evolution MUST follow a versioned proposal process. A compiler MUST reject unsupported language editions rather than silently reinterpret syntax.

**LANG-013** The grammar MUST support declarations, expressions, functions, structured control flow, modules, generic declarations, types, effects, error handling, concurrency constructs, memory-domain declarations and explicit unsafe regions.

**LANG-014** Formatting and syntactic sugar MUST NOT alter semantics. Desugaring MUST preserve source mapping and diagnostic provenance.

The initial release SHOULD prefer a compact grammar over extensive syntactic novelty. Surface syntax is subordinate to a coherent semantic model.

## 4. Type system

Aether SHALL use static type checking. The type system MUST specify primitive types, aggregates, tuples, enums/sum types, function types, generic types, references, pointers, arrays/slices, traits or interfaces, and foreign types.

**LANG-020** Type checking MUST complete before native code generation for the checked compilation unit.

**LANG-021** Generic instantiation MUST be deterministic for a fixed source, compiler version and configuration. The compiler MUST detect recursive instantiation and enforce resource limits.

**LANG-022** Implicit conversions MUST be limited and specified. Potentially lossy or representation-changing conversions SHOULD require explicit syntax.

**LANG-023** Nullability MUST be represented in types or an equally explicit mechanism; safe references MUST NOT be implicitly nullable.

**LANG-024** Mutability, aliasing and reference permissions MUST be represented sufficiently to enforce the memory model. The language MUST distinguish exclusive mutable access from shared read access in safe code.

**LANG-025** Type layout and ABI guarantees MUST be explicit. The compiler MUST NOT assume a stable in-memory layout for types not declared with an appropriate representation contract.

**LANG-026** Type inference MUST be deterministic and diagnostics MUST identify ambiguous or underconstrained expressions.

The initial implementation SHOULD use nominal declarations by default, structural rules only where explicitly specified, and a deliberately bounded generic system. Trait coherence, specialization and variance rules MUST be specified before implementation is treated as stable.

## 5. Effects and errors

The language MUST define effects for at least mutation, I/O, allocation, concurrency and unsafe operations, whether through explicit effect annotations, inferred effect sets or a combination.

**LANG-030** Effect inference MUST be deterministic and exposed to programme analysis.

**LANG-031** A transformation MUST preserve effect ordering and exception/error behaviour.

**LANG-032** The language MUST define a standard error propagation mechanism. Panics, recoverable errors and process termination MUST be distinct semantic outcomes.

**LANG-033** Destructors and cleanup MUST execute according to specified rules on normal return and supported error paths. The language MUST define which abrupt termination cases do not guarantee cleanup.

## 6. Memory and ownership integration

The language MUST expose memory-domain semantics defined in the Memory and Ownership Specification. A type's storage domain, ownership permissions and lifetime obligations MUST be available to the frontend and AIR.

**LANG-040** Safe references MUST have statically valid lifetime relationships or use a checked managed handle model.

**LANG-041** Raw pointer operations MUST require unsafe authority or a separately specified safe abstraction.

**LANG-042** Domain transitions MUST be explicit in semantics, even where syntax is inferred or lowered by the compiler.

**LANG-043** The compiler MUST NOT change a value's memory domain solely because a profile predicts better performance unless the transformation is semantics-preserving and satisfies the relevant ownership and domain rules.

## 7. Concurrency semantics

The language MUST define tasks, thread/task ownership, shared state, synchronisation, atomics, channels or equivalent message-passing primitives, cancellation and memory ordering.

**LANG-050** Safe code MUST prevent data races under the specified memory model.

**LANG-051** Atomic operations MUST specify supported orderings and legal combinations.

**LANG-052** Task cancellation MUST define resource cleanup and whether cancellation is cooperative, deferred or immediate at specified safe points.

**LANG-053** The compiler MUST preserve synchronisation and happens-before relationships.

## 8. Modules, packages and visibility

**LANG-060** Modules MUST have explicit identity, visibility and import rules. Cyclic dependencies MUST either be rejected or assigned fully specified semantics.

**LANG-061** Package resolution MUST be lockable and reproducible. A build MUST identify exact dependency versions and content digests.

**LANG-062** Public interfaces MUST be distinguishable from implementation details. The compiler MUST produce machine-readable interface metadata for incremental compilation and tooling.

**LANG-063** Module initialisation order MUST be deterministic or explicitly constrained. Hidden order-dependent initialisation MUST NOT be permitted.

**LANG-064** Conditional compilation MUST be based on declared target/configuration facts and MUST be captured in build identity.

## 9. ABI and foreign-function interface

The ABI specification MUST define calling conventions, symbol naming, type layout, alignment, aggregate passing, unwind/error boundaries, thread-local behaviour and ownership obligations for foreign values.

**LANG-070** Foreign declarations MUST declare or inherit explicit representation and ownership contracts.

**LANG-071** Safe wrappers MUST validate foreign inputs and isolate unsafe assumptions.

**LANG-072** ABI stability MUST be promised only for explicitly stable interfaces and specified language/ABI versions.

## 10. Diagnostics and tooling contract

Compiler diagnostics MUST have stable machine-readable codes, severity, source spans, notes and remediation context. The compiler MUST expose typed semantic information and source mappings to IDEs, debuggers and programme analysis tools.

## 11. Failure handling and security

Malformed source, invalid types, resource exhaustion during compile-time evaluation, dependency ambiguity and unsupported target features MUST produce deterministic diagnostics. The compiler MUST impose limits on nesting, generic expansion, compile-time execution and generated artefact size. Package metadata and compiler plugins MUST be treated as untrusted input.

## 12. Testing and acceptance

Required suites: lexer/parser conformance, grammar fuzzing, type-system positive/negative tests, evaluation-order tests, effect tests, lifetime and ownership tests, concurrency memory-model tests, module-resolution tests, ABI interoperability tests and cross-target semantic tests.

**Acceptance gate:** No stable language release until the semantics are written independently of implementation, conformance tests exist for every normative rule, the ABI is versioned, and safe-code guarantees are testable. Any unresolved semantic ambiguity blocks release rather than being left to compiler discretion.

---

# 2. Aether Memory and Ownership Specification
## Memory domains, ownership, garbage collection and domain transitions

**Status:** Normative memory-model baseline

## 1. Objectives

Aether SHALL support multiple memory-management strategies in one application without making safety dependent on the intelligence engine. Memory domains are language/runtime concepts with explicit semantics. The compiler may select or recommend a domain only when the programme's declared semantics and verification evidence permit it.

## 2. Memory domains

The initial domain taxonomy is:

1. **Static:** compile-time or process-lifetime storage with defined initialisation and destruction rules.
2. **Stack/owned:** lexically or structurally scoped storage with exclusive ownership and deterministic destruction.
3. **Region/arena:** groups of allocations reclaimed as a unit; individual object destruction rules MUST be explicit.
4. **Reference-counted:** shared ownership with specified atomicity, cycle behaviour and reclamation semantics.
5. **Tracing-managed:** objects reclaimed by a precise tracing collector when unreachable from defined roots.
6. **Shared/external:** shared memory, memory-mapped files, device memory and foreign resources governed by explicit external lifetime and access contracts.

**MEM-001** Every allocation MUST have a domain identity and an allocation contract.

**MEM-002** Every reference or handle MUST have a defined validity rule, ownership permission and reclamation relationship.

**MEM-003** An implementation MUST NOT silently reinterpret a raw pointer as a managed reference or vice versa.

**MEM-004** Domain selection MUST NOT weaken the guarantees of safe code.

## 3. Ownership and borrowing

The baseline safe model SHALL support unique ownership, shared immutable borrowing and exclusive mutable borrowing. The exact borrow syntax is language-defined; the semantic rules are mandatory.

**MEM-010** At most one exclusive mutable access path may be active for a location at a time in safe code.

**MEM-011** Shared borrows MUST NOT permit mutation except through explicitly specified interior-mutability or synchronisation abstractions.

**MEM-012** A borrow MUST NOT outlive the storage or owner that guarantees its validity.

**MEM-013** Ownership transfer MUST invalidate the prior exclusive owner unless the value is explicitly shared through a valid mechanism.

**MEM-014** Destruction MUST occur exactly according to the type/domain contract. Double destruction and use after destruction MUST be prevented in safe code.

The compiler MUST track provenance and aliasing through AIR. Conservative rejection is preferable to unsound acceptance.

## 4. Domain transition model

Legal transition mechanisms are:

- **Copy:** creates an independently valid value; copy semantics and cost MUST be explicit.
- **Move/transfer:** transfers ownership and invalidates the source ownership capability.
- **Immutable borrow:** permits read access while the source domain guarantees lifetime.
- **Managed handle:** indirect reference whose validity is mediated by a runtime/domain manager.
- **Pin:** prevents relocation while a valid pin capability exists; pin acquisition/release MUST be balanced.
- **Foreign transfer:** crosses an ABI boundary under a declared ownership and destruction contract.

**MEM-020** Every cross-domain transition MUST be represented in semantic analysis and AIR.

**MEM-021** A transition MUST specify source domain, destination domain, ownership effect, lifetime, mutability, failure mode and cleanup responsibility.

**MEM-022** A managed reference MUST NOT become an untracked raw pointer into a movable heap.

**MEM-023** A region object MUST NOT escape the region's lifetime unless copied, moved into a longer-lived domain under a valid rule, or represented by a safe managed handle.

**MEM-024** A reference-counted cycle MUST have specified treatment. The initial implementation MAY require explicit weak references or cycle-breaking mechanisms.

**MEM-025** Shared/external resources MUST define who owns release, mapping, unmapping and device synchronisation.

## 5. Garbage collection

The first tracing collector SHOULD be precise, non-moving mark-and-sweep, limited to explicitly managed domains. It MUST NOT scan arbitrary machine words and assume every pointer-shaped value is a root.

The collector contract MUST define:
- Root sources: registered stacks, globals, handles, runtime roots and foreign roots.
- Object metadata and allocation layout.
- Pointer maps or equivalent precise tracing metadata.
- Write barriers where required by the selected collector.
- Thread safepoints and mutator coordination.
- Finalisation/destructor semantics and restrictions.
- Behaviour under allocation failure.
- Interaction with pinned objects and foreign references.

**MEM-030** Reachable managed objects MUST NOT be reclaimed.

**MEM-031** Every managed reference path required for reachability MUST be visible to the collector or represented by a registered handle.

**MEM-032** Collector metadata corruption MUST be treated as a fatal runtime integrity failure, not silently ignored.

**MEM-033** Finalisers MUST NOT be relied upon for timely release of critical external resources. Deterministic cleanup APIs MUST exist.

**MEM-034** The initial collector MUST define whether collection is stop-the-world and its safepoint protocol. Concurrent or generational collection MUST be introduced only with a separately verified barrier and root model.

## 6. Runtime and compiler interfaces

The memory subsystem SHALL expose conceptual interfaces for:
- Domain creation/destruction.
- Allocation/deallocation.
- Ownership transfer.
- Borrow validation where runtime checks are required.
- Handle creation/resolution/release.
- Pin/unpin.
- Root registration.
- Collection request/status.
- Allocation and reclamation profiling.

Requests MUST carry domain/type metadata as required. Operations MUST return typed success/failure outcomes; allocator failure MUST never yield an invalid object.

## 7. Memory intelligence

The Memory Intelligence component MAY rank strategies using lifetime, escape, sharing, size, allocation frequency, destruction requirements, locality and fragmentation features.

The cost model SHOULD account for:
`allocation + reclamation + fragmentation + metadata + locality + coordination`.

Static proofs and runtime observations MUST remain separate. For example, a statically proven non-escape fact may justify a region transformation; an observed short lifetime alone is only evidence for a candidate and MUST NOT justify an unsafe lifetime shortening.

## 8. Failure handling

Allocation failure, region exhaustion, reference-count overflow, collector failure, invalid handle, pin failure and external resource failure MUST have defined outcomes. Corrupt heap metadata, impossible ownership state or violated internal invariants MUST trigger a fail-stop or documented containment path. Recovery MUST NOT continue with potentially corrupted memory state.

## 9. Security

The runtime MUST validate sizes, alignment, arithmetic overflow and metadata bounds. Handles MUST resist stale-handle reuse (for example, generation counters). Unsafe FFI boundaries MUST be audited. The collector and allocator MUST be fuzzed with adversarial object graphs and allocation patterns. Debug metadata MUST not expose secrets.

## 10. Testing and acceptance

Test ownership transfer, borrow exclusivity, lifetime rejection, destructor order, region escape, RC cycles, precise root enumeration, stack/register roots, write barriers, pinned objects, foreign handles, OOM, fragmentation and concurrent stress.

**Acceptance gate:** Safe-code memory guarantees must be enforced by the language and verifier; all implemented domain transitions must be specified and tested; the initial GC must pass root/reachability and adversarial stress tests. No adaptive memory strategy may be enabled before these gates pass.

---

# 3. Aether Intermediate Representation Specification
## AIR structures, invariants and transformation rules

**Status:** Normative compiler-internal contract

## 1. Purpose

Aether Intermediate Representation (AIR) is the compiler's versioned semantic representation. It preserves enough type, ownership, effect, memory-domain, concurrency and source provenance information to support whole-program analysis, verification, optimisation and native code generation.

AIR is not merely a renamed SSA form. SSA is a useful value discipline within AIR, but AIR additionally models memory and concurrency contracts explicitly.

## 2. Layered representation

AIR SHALL support the following logical stages:

- **AIR-Syntax:** source-oriented nodes, spans and desugaring provenance.
- **AIR-Typed:** resolved symbols, types, generics, control-flow graph and effects.
- **AIR-Memory:** allocations, ownership capabilities, borrow/lifetime relations, domain identity and transitions.
- **AIR-Concurrent:** tasks, channels, atomics, synchronisation and happens-before-relevant operations.
- **AIR-Optimised:** verified transformations and optimisation provenance.
- **AIR-Target:** target-specific operations, constraints and machine features.

A compiler MAY use multiple physical encodings, but MUST preserve equivalent information and provide versioned serialisation where AIR is persisted or exchanged.

## 3. Core structures

An AIR module MUST contain:
- Module identity and source/build metadata.
- Type and symbol tables.
- Functions and function signatures.
- Basic blocks and terminators.
- SSA values and explicit value-use relationships.
- Memory operations and alias/provenance metadata.
- Ownership and lifetime facts.
- Memory-domain declarations and transition operations.
- Effect summaries.
- Task, synchronisation and atomic operations.
- Target constraints where applicable.
- Debug/source mappings.
- Transformation and verification provenance.

Functions MUST have explicit entry points, control-flow edges and terminators. Every value MUST have a well-defined type and definition. External declarations MUST include effect and ABI contracts.

## 4. Required invariants

**AIR-001 — SSA validity:** each SSA value has one definition, all uses are dominated by the definition, and phi/block-argument inputs match predecessor structure.

**AIR-002 — CFG validity:** every block is reachable or explicitly marked as intentionally retained; terminators and successor edges are consistent.

**AIR-003 — Type validity:** every operation's operands and result conform to its declared type rule.

**AIR-004 — Ownership validity:** moves, borrows, drops and transfers obey the language ownership model.

**AIR-005 — Domain validity:** every memory access is valid for the object's domain and the active capability.

**AIR-006 — Lifetime validity:** no safe reference or handle is used outside its validity interval.

**AIR-007 — Effect validity:** transformations preserve observable effects and required ordering.

**AIR-008 — Concurrency validity:** task lifetimes, synchronisation, atomics and shared access obey the language memory model.

**AIR-009 — Provenance validity:** transformed operations retain a trace to source or generated origin and the passes responsible.

**AIR-010 — Target validity:** target operations are supported by the selected target or have a defined lowering.

The verifier MUST report the invariant identifier and relevant IR/source locations.

## 5. Transformation contract

Every transformation pass MUST declare:
- Unique pass ID and version.
- Input AIR stage and supported schema versions.
- Preconditions.
- Expected postconditions.
- Invariants it may affect.
- Required analyses and invalidation rules.
- Determinism properties.
- Resource limits.
- Diagnostics and rollback behaviour.

**AIR-020** A pass MUST NOT mutate shared AIR concurrently unless the representation and mutation protocol explicitly support it.

**AIR-021** The verifier MUST run after every pass in verification/debug builds and at defined trust boundaries in production builds.

**AIR-022** A pass MUST invalidate or update analyses whose assumptions it changes.

**AIR-023** A failed pass MUST leave the last valid AIR intact or return an explicit unusable result. Partial mutation MUST NOT be mistaken for success.

**AIR-024** Pass ordering MUST be explicit and versioned in build provenance.

**AIR-025** A pass generated or proposed by an AI system MUST be treated as untrusted and cannot bypass the same verifier and test pipeline as a conventional pass.

## 6. Programme analysis metadata

AIR MUST support conservative call graph, data-flow, alias, escape, lifetime, effect, allocation, control-flow and concurrency summaries. Each fact MUST include scope, origin, confidence/proof category and invalidation dependencies.

Facts SHALL distinguish:
- **Proven:** derived by a sound static rule under stated assumptions.
- **Checked:** established by runtime check or validation in a defined execution.
- **Observed:** sampled or measured, not universally guaranteed.
- **Predicted:** estimated by a model.

The compiler MUST NOT treat Observed or Predicted facts as Proven.

## 7. Serialisation and compatibility

AIR serialisation MUST include schema version, producer identity, target assumptions and integrity digest. Readers MUST reject incompatible mandatory schema versions. Optional metadata MAY be ignored only where doing so cannot affect correctness. Persisted AIR MUST be treated as untrusted input and fully verified before use.

## 8. Failure handling and security

Malformed AIR, invalid references, oversized modules, recursive type graphs and hostile metadata MUST fail safely under resource limits. Deserialisation MUST validate lengths, indices, enum values and checksums before allocation or interpretation. A failed verification MUST block code generation for the affected candidate.

## 9. Testing and acceptance

Required tests include verifier unit tests for every invariant, malformed AIR fuzzing, pass precondition/postcondition tests, differential pass testing, translation validation where applicable, determinism tests, analysis invalidation tests and round-trip serialisation tests.

**Acceptance gate:** AIR cannot be used as a trusted code-generation input until all mandatory invariants are machine-checked and every production pass has a declared contract, regression suite and provenance record.

---

# 4. Aether Intelligence Engine Specification
## Agents, planning, strategy contracts and developer policy

**Status:** Normative autonomy and decision-governance contract

## 1. Purpose and limits

The Aether Intelligence Engine (AIE) discovers optimisation opportunities, generates structured strategy candidates, estimates costs, coordinates experiments and explains recommendations. It is not a semantic authority, code generator with unrestricted privileges, or safety verifier.

The six initial agents are logical components: Programme, Memory, Execution, Hardware, Performance and Research. They need not be separate processes or LLMs.

## 2. Components

- **Intelligence Coordinator:** receives requests, schedules analysis, applies policy and manages lifecycle.
- **Programme Agent:** consumes programme graph and compiler facts.
- **Memory Agent:** evaluates legal memory strategies.
- **Execution Agent:** evaluates parallelism, scheduling and synchronisation alternatives.
- **Hardware Agent:** evaluates target features, cache, SIMD and NUMA opportunities.
- **Performance Agent:** interprets profiles, estimates costs and evaluates measurements.
- **Research Agent:** proposes candidate ideas from known transformations, literature-derived knowledge and reviewed research input.
- **Strategy Planner:** composes compatible strategies and prunes invalid combinations.
- **Cost Model Registry:** resolves approved model versions and feature schemas.
- **Policy Manager:** authorises or rejects actions under developer policy.
- **Explainability Reporter:** emits evidence-based reasons, uncertainty and trade-offs.

## 3. Strategy data contract

A Strategy MUST contain:
- Stable strategy ID and schema version.
- Transformation identifier and implementation version.
- Scope: expression, function, module, application or runtime region.
- Preconditions and required analyses.
- Required target features.
- Memory/concurrency/effect implications.
- Safety obligations and verifier requirements.
- Resource and compile-time bounds.
- Expected metric changes and uncertainty.
- Evidence references and provenance.
- Compatibility/conflict declarations.
- Status: proposed, rejected, verified, measured, approved, applied or revoked.

Strategies MUST be immutable after publication. Changes create a new version and digest. Free-form natural language MUST NOT be executable strategy input.

## 4. Planning pipeline

**AIE-001** The coordinator MUST execute a defined pipeline: ingest facts; detect opportunity; generate strategies; validate preconditions; check compatibility; estimate costs; request deterministic verification; create candidate plans; request experiments where allowed; rank results; explain recommendation; obtain policy authorisation; record provenance.

**AIE-002** A strategy MUST be rejected if a required precondition is false, unknown where proof is required, or unsupported by the target.

**AIE-003** The planner MUST account for interactions among strategies. Independently beneficial transformations MUST NOT be assumed jointly beneficial.

**AIE-004** The planner MUST bound search depth, candidate count, CPU, memory, wall time and generated artefact size.

**AIE-005** Failure or timeout of an agent MUST produce an explicit unavailable/unknown result and MUST NOT be interpreted as approval.

## 5. Developer control modes

- **Observe:** analysis and recommendations only.
- **Experiment:** generate and benchmark candidates in isolation; do not alter production strategy.
- **Approve:** require an explicit approval record before application.
- **Enforce:** apply only strategies previously authorised by a versioned policy within its exact scope.

Production default SHALL be Approve. A developer MAY configure Enforce for a narrowly scoped set of previously validated strategies. No mode permits bypassing compiler/runtime safety checks.

Policies MUST support allowed memory domains, concurrency transformations, runtime adaptation, targets, CPU/memory/compile-time budgets, optimisation priorities, verification levels and forbidden operations. Policy changes MUST be versioned and included in build identity where they affect output.

## 6. Interfaces

AIE SHALL expose typed operations to:
- Submit ProgrammeIdentity and verified ProgrammeFacts.
- Request opportunity analysis.
- Generate candidate strategy sets.
- Evaluate compatibility and preconditions.
- Request cost estimates.
- Submit candidates to verification/experimentation.
- Retrieve ranked recommendations and explanations.
- Record approval/revocation.
- Query applied-strategy provenance.

All calls MUST be authenticated within the host process/service boundary where applicable, schema-validated, cancellable and resource-bounded.

## 7. Explainability

Every recommendation MUST disclose scope, baseline, proposed change, reason, expected benefit, uncertainty, evidence, safety result, policy result, resource cost and alternatives considered. Predicted gains MUST be labelled predicted; measured gains MUST include benchmark context. The system MUST explain why a candidate was rejected where a material reason is available.

## 8. Security and isolation

AIE MUST run with least privilege. Research content, model output, package metadata and profiles are untrusted inputs. AIE MUST NOT directly write production binaries, alter source files, access arbitrary secrets or execute arbitrary generated code. Experimental execution MUST be delegated to the isolated ACE environment. Model prompts and outputs MUST be logged/redacted according to privacy policy and MUST never grant privileges.

## 9. Failure handling

If models are unavailable, corrupted, incompatible or over budget, AIE MUST fall back to deterministic rules or return no recommendation. Invalid strategy schema, conflicting policies or missing approval MUST fail closed. Coordinator restart MUST recover only from integrity-checked persisted state.

## 10. Testing and acceptance

Test agent isolation, strategy schema validation, precondition failures, policy precedence, conflict detection, budget enforcement, timeout/cancellation, deterministic planning under pinned inputs, explainability completeness and fail-closed behaviour.

**Acceptance gate:** AIE is releasable only when no AI/model output can bypass verification or policy, every applied strategy has a complete immutable contract and provenance, and deterministic compilation remains functional with all intelligence components disabled.

---

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

---

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

---

# 7. Aether Safety and Verification Specification
## Correctness, static verification and validation

**Status:** Normative trust and correctness contract

## 1. Objective

This specification defines how Aether establishes that source programs and compiler transformations satisfy language rules. It does not claim that arbitrary program equivalence is decidable or that testing alone proves correctness.

## 2. Verification layers

1. Lexical, syntactic and type/semantic validation.
2. Ownership, lifetime and memory-domain verification.
3. Effect and concurrency verification.
4. AIR structural/invariant verification.
5. Transformation validation.
6. Backend/code-generation validation.
7. Runtime guard and deoptimisation validation.
8. Native executable testing and platform integration.

Each layer MUST state its assumptions, checked properties, limitations and evidence class.

## 3. Evidence classification

- **Proven:** established by a sound proof or static rule under explicit assumptions.
- **Checked:** established by a runtime check, translation validation or bounded verification event.
- **Observed:** seen in a test, benchmark or profile; not universal.
- **Predicted:** estimated by a model.

**SAFE-001** Evidence classes MUST be preserved in diagnostics, strategy records and provenance.

**SAFE-002** Observed or predicted evidence MUST NOT be represented as a proof.

**SAFE-003** A verifier failure or unknown result for a required property MUST block the candidate.

## 4. Semantic correctness

The compiler MUST preserve all observable behaviour defined by the language specification, including side effects, error behaviour, memory ordering, destruction obligations and foreign-interface contracts.

The verifier SHOULD use translation validation for high-risk transformations where feasible: validate the particular transformed result against the relevant source/IR semantics. This does not imply full equivalence checking for arbitrary programs.

**SAFE-010** Every optimisation pass MUST declare the semantic properties it relies upon and preserves.

**SAFE-011** Any transformation whose preconditions cannot be established MUST be rejected or restricted to a runtime-guarded form with a correct fallback.

## 5. Memory and concurrency safety

Safe code MUST prevent use-after-free, dangling references, double destruction, invalid uninitialised reads, illegal aliasing, invalid domain transitions, reclamation of reachable managed objects and data races under the language memory model.

Unsafe code MUST have explicit boundaries and documented proof obligations. The compiler MUST NOT infer that unsafe code is safe merely because tests pass.

Concurrency verification SHOULD combine static checks, runtime instrumentation, stress tests and sanitiser integration. Limitations MUST be documented; absence of a detected race is not proof of race freedom unless the analysis is sound for the relevant programme.

## 6. Verification interfaces

A VerificationRequest MUST include candidate/AIR digest, verifier version, target assumptions, required properties, policy level and resource limits.

A VerificationResult MUST include pass/fail/unknown, properties checked, assumptions, diagnostics, proof/check artefacts, verifier version, resource use and digest. Results MUST be bound to the exact candidate digest.

A cached result MAY be reused only when all relevant inputs and verifier versions match exactly.

## 7. Native backend validation

Backend validation MUST include ABI tests, instruction legality, register/stack correctness, relocation/linking tests, unwind/debug metadata tests and target-feature validation. Differential testing against a trusted reference implementation SHOULD be used for supported subsets. Sanitiser and hardware-assisted testing SHOULD be integrated where available.

## 8. Testing strategy

Required test families:
- Language conformance and negative tests.
- Property-based and metamorphic tests.
- Parser, AIR and binary-format fuzzing.
- Differential compilation and execution.
- Memory and concurrency stress tests.
- Fault injection and OOM tests.
- Cross-architecture tests.
- Regression corpus from every fixed defect.
- Compiler self-hosting tests once self-hosting exists.

Fuzzers MUST have resource bounds and minimise reproducible failures. Every correctness defect MUST add a regression test.

## 9. Failure handling and security

Verification timeout or resource exhaustion MUST return Unknown, not Pass. Crashes, corrupted proof artefacts or digest mismatch MUST invalidate the result. Verifier inputs are untrusted and MUST be bounds-checked. Verification workers SHOULD be sandboxed when processing externally supplied IR or packages.

## 10. Acceptance

A transformation is eligible for production only when its required properties are checked, all relevant tests pass, its limitations are documented and provenance is complete. Safe-language release requires memory and concurrency semantics to be enforced, not merely described. Aether MUST publish a clear statement of what is proven, checked, observed and unsupported.

---

# 8. Aether Learning System Specification
## Training data, models, inference and model governance

**Status:** Normative ML governance contract

## 1. Purpose

The Aether Learning System (ALS) learns cost and strategy-selection relationships from compiler features, hardware characteristics and measured outcomes. ML improves prioritisation and search efficiency; it does not establish semantic correctness or safety.

Aether MUST remain usable offline and without third-party AI APIs. Python MAY be used for offline research/training; production compiler and runtime operation MUST NOT require Python.

## 2. Model families

Initial model families MAY include:
- Memory strategy prediction.
- Execution strategy prediction.
- Hardware cost estimation.
- Optimisation selection/ranking.
- Resource and compilation-cost prediction.

Initial deployment SHOULD favour interpretable supervised models, such as gradient-boosted trees, plus analytical cost models. Bayesian optimisation MAY guide bounded experiments. Reinforcement learning and evolutionary methods SHOULD remain research-stage until safety, reproducibility and evaluation are mature.

## 3. Dataset schema

Each training record MUST include:
- Programme/IR feature schema version.
- Programme class and workload identity, with privacy-safe identifiers.
- Target hardware and operating environment.
- Candidate strategy IDs and versions.
- Compiler, runtime and model versions.
- Correctness/verification status.
- Compile time and resource consumption.
- Execution time, throughput and latency distribution.
- Peak memory, allocation rate, CPU and contention where measured.
- Binary size and energy only where reliable.
- Sample count, measurement uncertainty and experiment identity.
- Failure label and termination reason where applicable.

Failed candidates MUST be retained with precise labels such as invalid precondition, verifier failure, semantic mismatch, compile budget exceeded, poor performance, memory budget exceeded or target incompatibility. Failed data MUST NOT be mislabelled as a successful optimisation.

## 4. Data quality and privacy

**LEARN-001** Every record MUST be traceable to an experiment and candidate digest.

**LEARN-002** Training pipelines MUST reject corrupt, incomplete or schema-incompatible records or quarantine them with an explicit reason.

**LEARN-003** Duplicate and leakage checks MUST prevent train/test contamination by programme revision, benchmark input or closely related workload where such leakage would inflate results.

**LEARN-004** User source code, identifiers, profiles and telemetry MUST NOT be uploaded to external training services without explicit authorisation and a documented data policy.

**LEARN-005** Dataset access, export and deletion MUST be governed and audited.

## 5. Training and evaluation

Models MUST be evaluated on held-out programmes and hardware configurations where feasible. Evaluation MUST include calibration, error distribution, ranking quality, resource prediction error and performance relative to deterministic baselines.

A model that performs well only on training benchmarks MUST NOT be promoted. Evaluation MUST include ablation studies where useful and identify workloads on which the model is unreliable.

Model registry metadata MUST include model digest, training dataset digest, feature schema, training code/version, evaluation report, known limitations, supported targets and approval state.

## 6. Inference interface

Inference requests MUST be typed and versioned, with feature schema, target, strategy candidate set and resource budget. Responses MUST include predictions, uncertainty/confidence or calibration metadata, model identity and applicability status.

Unknown features, out-of-distribution inputs, incompatible model versions or inference timeout MUST result in a conservative fallback. A model MUST NOT fabricate missing hardware or programme facts.

## 7. Model governance

**LEARN-010** Model updates MUST be versioned and MUST NOT silently alter reproducible or production builds.

**LEARN-011** Production models MUST pass offline evaluation, compatibility checks, security review and approval.

**LEARN-012** Online learning MUST NOT directly modify the active production model. New observations may be queued for offline retraining and validation.

**LEARN-013** A model MUST be revocable and rollback to a previously approved version MUST be supported.

**LEARN-014** Model inference MUST be bounded by time and memory budgets.

## 8. LLM boundary

LLMs MAY assist with research summaries, explanations, candidate ideas and test suggestions. Their output MUST be treated as untrusted text. Any proposed transformation MUST be translated into a structured Strategy, implemented through trusted compiler code, verified and experimentally evaluated. LLMs MUST NOT authoritatively change semantics, approve policies, sign builds or inject executable code into trusted compiler/runtime components.

## 9. Security

Model files, training artefacts and datasets MUST be integrity-checked. Training pipelines MUST defend against poisoned records, label corruption and data leakage. Model loading MUST validate format and resource bounds. Secrets and raw user data MUST be excluded from features unless explicitly required and approved.

## 10. Testing and acceptance

Test feature schema compatibility, deterministic inference where supported, out-of-distribution fallback, calibration, dataset leakage, poisoned-data detection, model rollback, offline operation, resource limits and model-version pinning.

**Acceptance gate:** No model can influence a production strategy without approved model identity, evaluation evidence, deterministic safety gates and fallback. The compiler must remain correct and usable with ALS disabled.

---

# 9. Aether Reproducible Build Specification
## Build identities, provenance and deterministic output

**Status:** Normative build integrity contract

## 1. Objective

Aether builds MUST be identifiable, traceable and reproducible to the extent supported by the target toolchain and environment. Adaptive intelligence MUST NOT silently mutate a build's strategy after its identity has been established.

## 2. Build identity

BuildIdentity MUST cover all inputs that can affect generated output or strategy selection, including:
- Source revision and content digests.
- Module graph and exact dependency lock.
- Language edition/specification version.
- Compiler, optimiser, backend and linker versions.
- AIR schema/version and relevant AIR digest.
- Target architecture, ABI and enabled CPU features.
- Build flags and environment-derived configuration.
- Strategy IDs/versions and optimisation pass order.
- Model and cost-model versions where used.
- ProfileSnapshot identity where used.
- Developer policy digest.
- Relevant environment/toolchain identities.
- Determinism settings and build mode.

The identity MUST be cryptographically bound to the build manifest. Missing mandatory inputs MUST fail the build or be explicitly represented as non-reproducible.

## 3. Build modes

- **Reproducible:** pinned compiler, dependencies, strategies, profiles and models.
- **Adaptive:** may discover and evaluate new strategies, but candidate selection and evidence are recorded.
- **Production:** only approved, validated strategies under pinned policy/model versions.
- **Research:** broad bounded experimentation, isolated from production artefacts.

A build mode MUST be explicit. Production MUST NOT inherit Research permissions.

## 4. Determinism

**BUILD-001** Identical declared inputs SHOULD produce byte-identical artefacts where deterministic output is supported by the platform and toolchain.

**BUILD-002** Sources of nondeterminism such as timestamps, filesystem ordering, random seeds, parallel pass scheduling and environment probing MUST be normalised, pinned or recorded.

**BUILD-003** If byte-identical output cannot be guaranteed, the build MUST report the reason and provide semantic/input reproducibility metadata.

**BUILD-004** Runtime timings and adaptive decisions based on live scheduling or hardware noise are not expected to be identical; they MUST be recorded as observations.

## 5. Provenance

Every non-baseline optimisation MUST have provenance linking source facts, strategy, candidate, verifier result, experiment, approval and emitted artefact. Provenance MUST identify who or what authorised application and under which policy.

Provenance records MUST be immutable or tamper-evident, schema-versioned and integrity-checked. A build MUST be inspectable without requiring the original model service to remain available.

## 6. Caches and artefact reuse

Build caches MUST be keyed by complete relevant identities. A cache hit MUST validate integrity and compatibility. Stale, incomplete or mismatched entries MUST be discarded. Cache storage MUST be treated as untrusted input and validated before use.

## 7. Interfaces and outputs

The build system MUST emit:
- Build manifest.
- BuildIdentity digest.
- Compiler and dependency manifest.
- Strategy and policy manifest.
- Verification summary.
- Experiment references where relevant.
- Artefact digests.
- Reproducibility status and known limitations.

The CLI MUST support inspecting why a build has a given identity and reproducing it from pinned inputs.

## 8. Security and failure handling

Build manifests and provenance MUST be protected from unauthorised modification. Signing keys MUST be managed outside source repositories and accessed through a defined signing boundary. Dependency acquisition MUST validate content digests and source policy. Missing approval, signature mismatch or provenance corruption MUST block production promotion.

## 9. Testing and acceptance

Run clean-room rebuilds, cross-directory builds, parallel/sequential builds, cache corruption tests, dependency substitution tests, model-version pinning tests and manifest tampering tests.

**Acceptance gate:** Production artefacts must have complete identities and provenance. A build whose strategy cannot be reconstructed or whose policy approval cannot be verified MUST NOT be promoted.

---

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
