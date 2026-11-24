# Syntra Learning System (SLS)

The Syntra Learning System (SLS) is an empirical machine learning subsystem embedded into the toolchain (`crates/intelligence/`) that trains predictive models on execution telemetry to guide compiler optimizations.

---

## 🧠 Model Families

SLS maintains 5 specialized model families:

| Model Family | Target Prediction | Optimization Impact |
| :--- | :--- | :--- |
| `Memory` | Peak memory usage and allocation volume | Arena sizing and escape analysis decisions |
| `Execution` | Runtime speedup ($\Delta$) of optimization candidates | Optimization pass ordering and inlining |
| `HardwareCost` | Cache misses and branch mispredictions | Loop unrolling and data alignment |
| `Ranking` | Comparative preference between multiple candidates | Selection along Pareto frontiers |
| `ResourcePrediction` | CPU core saturation and thread contention | Concurrency thread pool sizing |

---

## 📐 Supervised Ensemble Architecture (`crates/intelligence/src/model.rs`)

SLS uses a pure-Rust, deterministic supervised ensemble model:
* **Calibrated Confidence Intervals:** Computes both point predictions and empirical continuous confidence intervals.
* **Feature Encodings:** Encodes loop nesting depth, call count, branch complexity, memory domain mode, and instruction density.
* **Zero External Dependencies:** Runs natively in Rust with zero Python, C++, or GPU dependencies.

---

## 🎯 Production Promotion Gate (`crates/intelligence/src/evaluation.rs`)

Before any newly trained model version can be promoted to active production use, it must satisfy strict empirical evaluation criteria:

```rust
pub struct EvaluationMetrics {
    pub rmse: f64,               // Root Mean Squared Error
    pub mae: f64,                // Mean Absolute Error
    pub spearman_rho: f64,       // Rank correlation coefficient
    pub expected_calibration_error: f64, // ECE calibration metric
    pub baseline_speedup_ratio: f64,
}
```

Promotion requirements:
1. **ECE Calibration:** Expected Calibration Error $\le 0.55$ for small sample sizes and $\le 0.15$ for production datasets.
2. **Speedup Superiority:** Model recommendations must outperform the heuristic baseline by at least $+5\%$.
3. **Correlation:** Spearman $\rho \ge 0.60$, ensuring reliable ranking of optimization candidates.

---

## 🔒 Dataset Governance & Privacy (`crates/intelligence/src/governance.rs`)

SLS enforces rigorous data governance rules:
* **Leakage-Free Splitting:** Training and test datasets are partitioned strictly by workload identifier (`workload_id`) rather than random rows, preventing data leakage.
* **Poison Data Quarantine:** Anomalous telemetry (negative latencies, NaN metrics, or extreme outliers) is quarantined in `.syntra/quarantine/` with signed reason codes (`QuarantineReason`).
* **Privacy-Safe Hashing:** All file paths, symbols, and identifier names are anonymized using one-way SHA-256 digests before training record ingestion.
* **Governance Audit Log:** All training runs, promotions, rollbacks, and revocations are immutably appended to `.syntra/governance_audit.log`.

---

## 🔄 Model Rollback & Revocation (`crates/intelligence/src/registry.rs`)

Model versions are tracked in `.syntra/models/registry.json`:
* `syn learn rollback <version>`: Atomically reverts active production tier to a prior version.
* `syn learn revoke <version>`: Marks a degraded model as permanently revoked, preventing accidental promotion.
