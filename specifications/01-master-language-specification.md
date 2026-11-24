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
