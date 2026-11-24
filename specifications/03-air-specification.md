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
