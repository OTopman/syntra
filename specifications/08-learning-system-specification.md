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
