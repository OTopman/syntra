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
