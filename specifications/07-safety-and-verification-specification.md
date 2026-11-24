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
