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
