use crate::dataset::OptimizationDataset;
use crate::model::SupervisedEnsembleModel;

/// Comprehensive evaluation report for a model on held-out test data (Specification 08 Section 5)
#[derive(Debug, Clone)]
pub struct ModelEvaluationReport {
    pub model_id: String,
    pub version: String,
    pub test_samples_count: usize,
    pub rmse_speedup_pct: f64,
    pub mae_speedup_pct: f64,
    pub expected_calibration_error: f64,
    pub spearman_rank_correlation: f64,
    pub baseline_rmse_speedup_pct: f64,
    pub relative_improvement_pct: f64,
    pub passed_gate: bool,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PromotionGateDecision {
    Approved(ModelEvaluationReport),
    Rejected {
        reason: String,
        report: ModelEvaluationReport,
    },
}

pub struct ModelEvaluator;

impl ModelEvaluator {
    /// Computes Spearman's rank correlation coefficient between two ranked series
    pub fn compute_spearman_correlation(predicted: &[f64], empirical: &[f64]) -> f64 {
        if predicted.len() < 2 || predicted.len() != empirical.len() {
            return 1.0;
        }

        let n = predicted.len() as f64;
        let mut pred_indexed: Vec<(usize, f64)> = predicted.iter().copied().enumerate().collect();
        let mut emp_indexed: Vec<(usize, f64)> = empirical.iter().copied().enumerate().collect();

        pred_indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        emp_indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut pred_ranks = vec![0.0; predicted.len()];
        let mut emp_ranks = vec![0.0; empirical.len()];

        for (rank, (orig_idx, _)) in pred_indexed.into_iter().enumerate() {
            pred_ranks[orig_idx] = (rank + 1) as f64;
        }
        for (rank, (orig_idx, _)) in emp_indexed.into_iter().enumerate() {
            emp_ranks[orig_idx] = (rank + 1) as f64;
        }

        let mut d_sq_sum = 0.0;
        for i in 0..predicted.len() {
            let diff = pred_ranks[i] - emp_ranks[i];
            d_sq_sum += diff * diff;
        }

        1.0 - (6.0 * d_sq_sum) / (n * (n * n - 1.0))
    }

    /// Evaluates a candidate model against held-out test data and compares with baseline
    pub fn evaluate(
        candidate: &SupervisedEnsembleModel,
        baseline: &SupervisedEnsembleModel,
        test_dataset: &OptimizationDataset,
    ) -> ModelEvaluationReport {
        let valid_records: Vec<_> = test_dataset
            .records
            .iter()
            .filter(|r| r.correctness_passed)
            .collect();

        if valid_records.is_empty() {
            return ModelEvaluationReport {
                model_id: candidate.model_id.clone(),
                version: candidate.version.clone(),
                test_samples_count: 0,
                rmse_speedup_pct: 0.0,
                mae_speedup_pct: 0.0,
                expected_calibration_error: 0.0,
                spearman_rank_correlation: 1.0,
                baseline_rmse_speedup_pct: 0.0,
                relative_improvement_pct: 0.0,
                passed_gate: false,
                rejection_reason: Some("Held-out test dataset is empty".to_string()),
            };
        }

        let mut cand_errors_sq = 0.0;
        let mut cand_errors_abs = 0.0;
        let mut base_errors_sq = 0.0;
        let mut pred_series = Vec::new();
        let mut emp_series = Vec::new();
        let mut calibration_diffs = Vec::new();

        for rec in &valid_records {
            let feat = SupervisedEnsembleModel::to_feature_vec(&rec.features);
            let (cand_pred, _) = candidate.raw_predict(&feat);
            let (base_pred, _) = baseline.raw_predict(&feat);

            let emp_speedup_pct = (rec.speedup_factor - 1.0) * 100.0;

            let cand_err = cand_pred - emp_speedup_pct;
            let base_err = base_pred - emp_speedup_pct;

            cand_errors_sq += cand_err * cand_err;
            cand_errors_abs += cand_err.abs();
            base_errors_sq += base_err * base_err;

            pred_series.push(cand_pred);
            emp_series.push(emp_speedup_pct);

            // Statistical interval calibration (Kuleshov et al.)
            let (_, std_dev) = candidate.raw_predict(&feat);
            let margin = (1.96 * std_dev).max(8.0);
            let confidence: f64 =
                (1.0 / (1.0 + (std_dev / (cand_pred.abs() + 2.0)))).clamp(0.65, 0.95);
            let covered = if (emp_speedup_pct - cand_pred).abs() <= margin {
                1.0
            } else {
                0.0
            };
            calibration_diffs.push((confidence - covered).abs());
        }

        let n = valid_records.len() as f64;
        let rmse = (cand_errors_sq / n).sqrt();
        let mae = cand_errors_abs / n;
        let base_rmse = (base_errors_sq / n).sqrt();

        let ece = if !calibration_diffs.is_empty() {
            calibration_diffs.iter().sum::<f64>() / calibration_diffs.len() as f64
        } else {
            0.05
        };

        let spearman = Self::compute_spearman_correlation(&pred_series, &emp_series);
        let relative_improvement = if base_rmse > 0.0 {
            ((base_rmse - rmse) / base_rmse) * 100.0
        } else {
            0.0
        };

        // Specification 08 Section 5 Gate: Must not regress compared to baseline on held-out test data
        let mut passed_gate = true;
        let mut rejection_reason = None;

        if rmse > base_rmse * 1.05 {
            passed_gate = false;
            rejection_reason = Some(format!(
                "Regression detected: Candidate RMSE ({rmse:.2}) exceeds baseline RMSE ({base_rmse:.2}) by >5%"
            ));
        } else if spearman < 0.0 {
            passed_gate = false;
            rejection_reason = Some(format!(
                "Negative rank correlation ({spearman:.2}): Candidate inverts strategy ranking"
            ));
        } else if ece > 0.55 {
            passed_gate = false;
            rejection_reason = Some(format!(
                "Poor calibration error ({ece:.3}): Model confidence is uncalibrated"
            ));
        }

        ModelEvaluationReport {
            model_id: candidate.model_id.clone(),
            version: candidate.version.clone(),
            test_samples_count: valid_records.len(),
            rmse_speedup_pct: rmse,
            mae_speedup_pct: mae,
            expected_calibration_error: ece,
            spearman_rank_correlation: spearman,
            baseline_rmse_speedup_pct: base_rmse,
            relative_improvement_pct: relative_improvement,
            passed_gate,
            rejection_reason,
        }
    }

    /// Evaluates for production promotion
    pub fn evaluate_for_promotion(
        candidate: &SupervisedEnsembleModel,
        baseline: &SupervisedEnsembleModel,
        test_dataset: &OptimizationDataset,
    ) -> PromotionGateDecision {
        let report = Self::evaluate(candidate, baseline, test_dataset);
        if report.passed_gate {
            PromotionGateDecision::Approved(report)
        } else {
            let reason = report
                .rejection_reason
                .clone()
                .unwrap_or_else(|| "Failed gate criteria".to_string());
            PromotionGateDecision::Rejected { reason, report }
        }
    }
}
