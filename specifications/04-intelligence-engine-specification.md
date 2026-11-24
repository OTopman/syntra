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
