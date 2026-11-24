pub mod benchmark;
pub mod candidate;
pub mod runner;
pub mod verifier;

use benchmark::{BenchmarkEvaluator, BenchmarkMetrics};
use candidate::{Candidate, CandidateGenerator};
use runner::SandboxRunner;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use syntra_air::ir::AirModule;
use verifier::{EquivalenceVerifier, VerificationResult};

#[derive(Debug, Clone)]
pub struct CandidateExperimentResult {
    pub candidate: Candidate,
    pub verification: VerificationResult,
    pub benchmark: Option<BenchmarkMetrics>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExperimentReport {
    pub run_id: String,
    pub source_file: String,
    pub baseline_id: String,
    pub baseline_metrics: Option<BenchmarkMetrics>,
    pub results: Vec<CandidateExperimentResult>,
}

impl ExperimentReport {
    pub fn format_report(&self) -> String {
        let mut out = String::new();
        out.push_str("=========================================================================================\n");
        out.push_str("              Syntra Autonomous Experimental Compilation Engine (SECE)                   \n");
        out.push_str("                        Whole-Program Multi-Objective Evaluation                         \n");
        out.push_str("=========================================================================================\n");
        out.push_str(&format!("• Run ID:           {}\n", self.run_id));
        out.push_str(&format!("• Source File:      {}\n", self.source_file));
        out.push_str(&format!("• Baseline Strategy:{}\n", self.baseline_id));
        out.push_str(&format!("• Candidates Tested:{}\n", self.results.len()));
        out.push_str("-----------------------------------------------------------------------------------------\n");
        out.push_str(
            format!(
                "{:<16} {:<18} {:<10} {:<14} {:<12} {:<10} {:<14}\n",
                "Candidate ID",
                "Strategy",
                "Status",
                "Mean Latency",
                "Speedup",
                "Size",
                "Pareto Optimal"
            )
            .as_str(),
        );
        out.push_str("-----------------------------------------------------------------------------------------\n");

        let baseline_mean = self
            .baseline_metrics
            .as_ref()
            .map(|m| m.mean_time_us)
            .unwrap_or(1.0);

        for res in &self.results {
            let status_str = if res.verification.is_valid {
                "VERIFIED"
            } else {
                "REJECTED"
            };

            let (latency_str, speedup_str, size_str, pareto_str) = match &res.benchmark {
                Some(bench) => {
                    let speedup = if bench.mean_time_us > 0.0 {
                        baseline_mean / bench.mean_time_us
                    } else {
                        1.0
                    };
                    let lat = format!("{:.1} µs", bench.mean_time_us);
                    let spd = format!("{speedup:.2}x");
                    let sz = format!("{:.1} KB", bench.binary_size_bytes as f64 / 1024.0);
                    let par = if bench.is_pareto_optimal {
                        "★ OPTIMAL"
                    } else {
                        "  -"
                    };
                    (lat, spd, sz, par.to_string())
                }
                None => (
                    "N/A".to_string(),
                    "N/A".to_string(),
                    "N/A".to_string(),
                    "  -".to_string(),
                ),
            };

            out.push_str(&format!(
                "{:<16} {:<18} {:<10} {:<14} {:<12} {:<10} {:<14}\n",
                res.candidate.id,
                res.candidate.name,
                status_str,
                latency_str,
                speedup_str,
                size_str,
                pareto_str
            ));

            if let Some(err) = &res.error {
                out.push_str(&format!("    ↳ Pruned: {err}\n"));
            } else if let Some(reason) = &res.verification.failure_reason {
                out.push_str(&format!("    ↳ Verification Failure: {reason}\n"));
            }
        }

        out.push_str("-----------------------------------------------------------------------------------------\n");
        out.push_str("Multi-Objective Recommendation:\n");
        // Find best speedup among pareto optimal
        let best_opt = self
            .results
            .iter()
            .filter(|r| {
                r.benchmark
                    .as_ref()
                    .map(|b| b.is_pareto_optimal)
                    .unwrap_or(false)
            })
            .max_by(|a, b| {
                let spd_a = a
                    .benchmark
                    .as_ref()
                    .map(|m| baseline_mean / m.mean_time_us.max(0.001))
                    .unwrap_or(0.0);
                let spd_b = b
                    .benchmark
                    .as_ref()
                    .map(|m| baseline_mean / m.mean_time_us.max(0.001))
                    .unwrap_or(0.0);
                spd_a
                    .partial_cmp(&spd_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(rec) = best_opt {
            out.push_str(&format!(
                "  ➤ Recommended Candidate: {} ({})\n",
                rec.candidate.id, rec.candidate.name
            ));
            out.push_str(&format!("    Description: {}\n", rec.candidate.description));
            out.push_str(&format!(
                "    To commit this strategy for production builds, run:\n    syn approve {}\n",
                rec.candidate.id
            ));
        } else {
            out.push_str(
                "  ➤ Retaining baseline (no candidates strictly improved over baseline).\n",
            );
        }

        out.push_str("=========================================================================================\n");
        out
    }
}

pub struct ExperimentalCompilationEngine;

impl ExperimentalCompilationEngine {
    pub fn run_experiment(
        source_file: &str,
        baseline_air: &AirModule,
        iterations: usize,
    ) -> Result<ExperimentReport, String> {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let run_id = format!("ace_run_{ts}");
        let runner = SandboxRunner::new(run_id.clone());

        // 1. Generate Candidates
        let candidates = CandidateGenerator::generate_candidates(baseline_air);
        if candidates.is_empty() {
            return Err("Candidate generator produced no candidates".to_string());
        }

        let baseline_cand = &candidates[0];
        let baseline_id = baseline_cand.id.clone();

        // 2. Validate Baseline Invariants and Compile Baseline
        EquivalenceVerifier::verify_air_invariants(baseline_cand)
            .map_err(|e| format!("Baseline AIR failed invariant checks: {e}"))?;

        let baseline_compile = runner.compile_candidate(baseline_cand)?;
        let baseline_output = runner.execute_binary(&baseline_compile.binary_path, &[])?;
        if baseline_output.timed_out {
            return Err(
                "Baseline execution timed out in sandbox (exceeded time limit)".to_string(),
            );
        }

        let mut results = Vec::new();
        let mut baseline_metrics = None;

        // 3. Process and Benchmark Candidates
        for candidate in &candidates {
            // Check AIR Invariants
            if let Err(inv_err) = EquivalenceVerifier::verify_air_invariants(candidate) {
                results.push(CandidateExperimentResult {
                    candidate: candidate.clone(),
                    verification: VerificationResult {
                        is_valid: false,
                        air_invariants_passed: false,
                        differential_equivalence_passed: false,
                        failure_reason: Some(inv_err.clone()),
                    },
                    benchmark: None,
                    error: Some(inv_err),
                });
                continue;
            }

            // Compile Candidate in isolated sandbox
            let comp_res = runner.compile_candidate(candidate);
            let comp_out = match comp_res {
                Ok(out) => out,
                Err(err) => {
                    results.push(CandidateExperimentResult {
                        candidate: candidate.clone(),
                        verification: VerificationResult {
                            is_valid: false,
                            air_invariants_passed: true,
                            differential_equivalence_passed: false,
                            failure_reason: Some(format!("Compilation failed: {err}")),
                        },
                        benchmark: None,
                        error: Some(err),
                    });
                    continue;
                }
            };

            // Run Candidate once for differential testing
            let run_res = runner.execute_binary(&comp_out.binary_path, &[]);
            let run_out = match run_res {
                Ok(out) => out,
                Err(err) => {
                    results.push(CandidateExperimentResult {
                        candidate: candidate.clone(),
                        verification: VerificationResult {
                            is_valid: false,
                            air_invariants_passed: true,
                            differential_equivalence_passed: false,
                            failure_reason: Some(format!("Execution failed: {err}")),
                        },
                        benchmark: None,
                        error: Some(err),
                    });
                    continue;
                }
            };

            // Differential Equivalence Check (ACE-001)
            let verif = EquivalenceVerifier::verify_differential_equivalence(
                candidate,
                &baseline_output,
                &run_out,
            );

            if !verif.is_valid {
                results.push(CandidateExperimentResult {
                    candidate: candidate.clone(),
                    verification: verif,
                    benchmark: None,
                    error: Some("Differential equivalence verification failed".to_string()),
                });
                continue;
            }

            // Benchmark execution (ACE-010, ACE-012)
            let bench_res = BenchmarkEvaluator::evaluate_binary(
                &comp_out.binary_path,
                iterations.max(3),
                comp_out.binary_size_bytes,
                comp_out.compile_duration,
                |path| runner.execute_binary(path, &[]),
            );

            let bench_metrics = match bench_res {
                Ok(m) => {
                    if candidate.id == baseline_id {
                        baseline_metrics = Some(m.clone());
                    }
                    Some(m)
                }
                Err(e) => {
                    eprintln!("Warning: benchmark error for {}: {}", candidate.id, e);
                    None
                }
            };

            results.push(CandidateExperimentResult {
                candidate: candidate.clone(),
                verification: verif,
                benchmark: bench_metrics,
                error: None,
            });
        }

        // 4. Compute Pareto Frontier across verified candidates (ACE-014)
        let mut pareto_inputs: Vec<(String, BenchmarkMetrics)> = results
            .iter()
            .filter_map(|r| {
                r.benchmark
                    .as_ref()
                    .map(|b| (r.candidate.id.clone(), b.clone()))
            })
            .collect();

        BenchmarkEvaluator::compute_pareto_frontier(&mut pareto_inputs);

        for (cand_id, updated_metrics) in pareto_inputs {
            if let Some(r) = results.iter_mut().find(|r| r.candidate.id == cand_id) {
                if let Some(b) = &mut r.benchmark {
                    b.is_pareto_optimal = updated_metrics.is_pareto_optimal;
                }
            }
        }

        // Export trials to SLS Optimization Dataset (Subsystem 7 -> Subsystem 10)
        let dataset_path = PathBuf::from(".syntra/datasets/training_records.jsonl");
        let dataset = syntra_intelligence::OptimizationDataset::new();
        let features = syntra_intelligence::ProgramFeatures::extract_from_air(baseline_air);
        let base_mean = baseline_metrics
            .as_ref()
            .map(|b| b.mean_time_us)
            .unwrap_or(1.0);

        for r in &results {
            let cand_digest = format!(
                "{:016x}",
                r.candidate.air_module.functions.len() * 31 + r.candidate.id.len()
            );
            let prog_digest = syntra_intelligence::privacy_safe_identifier(source_file);
            let failure_reason = if !r.verification.is_valid {
                Some(syntra_intelligence::FailureReason::VerifierFailure)
            } else {
                r.error
                    .as_deref()
                    .map(syntra_intelligence::FailureReason::from_str)
            };

            let record = syntra_intelligence::TrainingRecord {
                feature_schema_version: "1.0.0".to_string(),
                experiment_id: run_id.clone(),
                candidate_digest: cand_digest,
                program_digest: prog_digest.clone(),
                workload_id: prog_digest,
                features: features.clone(),
                target_os: std::env::consts::OS.to_string(),
                target_arch: std::env::consts::ARCH.to_string(),
                candidate_id: r.candidate.id.clone(),
                strategy_name: r.candidate.name.clone(),
                correctness_passed: r.verification.is_valid,
                compile_time_ms: r.benchmark.as_ref().map(|b| b.compile_time_ms).unwrap_or(0),
                mean_latency_us: r
                    .benchmark
                    .as_ref()
                    .map(|b| b.mean_time_us)
                    .unwrap_or(1.0)
                    .max(1.0),
                speedup_factor: r
                    .benchmark
                    .as_ref()
                    .map(|b| (base_mean / b.mean_time_us.max(0.001)).clamp(0.01, 100.0))
                    .unwrap_or(1.0),
                binary_size_bytes: r
                    .benchmark
                    .as_ref()
                    .map(|b| b.binary_size_bytes)
                    .unwrap_or(0),
                sample_count: iterations.max(1),
                measurement_uncertainty: 0.02,
                is_pareto_optimal: r
                    .benchmark
                    .as_ref()
                    .map(|b| b.is_pareto_optimal)
                    .unwrap_or(false),
                failure_reason,
            };
            let _ = dataset.append_to_file(&record, &dataset_path);
        }

        Ok(ExperimentReport {
            run_id,
            source_file: source_file.to_string(),
            baseline_id,
            baseline_metrics,
            results,
        })
    }

    /// Commits an approved candidate strategy into .syntra/approved_strategy.json
    pub fn approve_candidate(candidate_id: &str, target_dir: &Path) -> Result<PathBuf, String> {
        let syntra_dir = target_dir.join(".syntra");
        fs::create_dir_all(&syntra_dir)
            .map_err(|e| format!("Failed to create .syntra directory: {e}"))?;

        let approval_path = syntra_dir.join("approved_strategy.json");
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let content = format!(
            r#"{{
  "approved_candidate_id": "{candidate_id}",
  "timestamp": {ts},
  "status": "APPROVED",
  "policy": "DeveloperExplicitApproval"
}}
"#
        );

        fs::write(&approval_path, content)
            .map_err(|e| format!("Failed to write approved_strategy.json: {e}"))?;

        Ok(approval_path)
    }
}

/// Represents a developer-approved optimization strategy manifest from `.syntra/approved_strategy.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedStrategy {
    pub approved_candidate_id: String,
    pub timestamp: u64,
    pub status: String,
    pub policy: String,
}

impl ApprovedStrategy {
    pub fn parse(content: &str) -> Option<Self> {
        let mut approved_candidate_id = String::new();
        let mut timestamp = 0u64;
        let mut status = String::new();
        let mut policy = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("\"approved_candidate_id\":") {
                if let Some(val) = line.split(':').nth(1) {
                    approved_candidate_id = val
                        .trim()
                        .trim_matches(|c| c == '"' || c == ',' || c == ' ')
                        .to_string();
                }
            } else if line.starts_with("\"timestamp\":") {
                if let Some(val) = line.split(':').nth(1) {
                    timestamp = val
                        .trim()
                        .trim_matches(|c| c == ',' || c == ' ')
                        .parse()
                        .unwrap_or(0);
                }
            } else if line.starts_with("\"status\":") {
                if let Some(val) = line.split(':').nth(1) {
                    status = val
                        .trim()
                        .trim_matches(|c| c == '"' || c == ',' || c == ' ')
                        .to_string();
                }
            } else if line.starts_with("\"policy\":") {
                if let Some(val) = line.split(':').nth(1) {
                    policy = val
                        .trim()
                        .trim_matches(|c| c == '"' || c == ',' || c == ' ')
                        .to_string();
                }
            }
        }

        if !approved_candidate_id.is_empty() && status == "APPROVED" {
            Some(ApprovedStrategy {
                approved_candidate_id,
                timestamp,
                status,
                policy,
            })
        } else {
            None
        }
    }

    pub fn load_from_dir(dir: &Path) -> Option<Self> {
        let path = dir.join(".syntra").join("approved_strategy.json");
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Option<Self> {
        if !path.is_file() {
            return None;
        }
        let content = fs::read_to_string(path).ok()?;
        Self::parse(&content)
    }

    pub fn to_strategy_kind(&self) -> candidate::StrategyKind {
        let id = self.approved_candidate_id.to_lowercase();
        if id.starts_with("cand_0") || id.contains("baseline") {
            candidate::StrategyKind::Baseline
        } else if id.starts_with("cand_1") || id.contains("escape") {
            candidate::StrategyKind::EscapePromoted
        } else if id.starts_with("cand_2") || id.contains("constant") || id.contains("fold") {
            candidate::StrategyKind::ConstantFolded
        } else if id.starts_with("cand_3") || id.contains("aggressive") || id.contains("combined") {
            candidate::StrategyKind::AggressiveCombined
        } else {
            candidate::StrategyKind::Baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntra_air::lower_and_verify;

    #[test]
    fn test_candidate_generation() {
        let code = r#"
func add(int a, int b): int {
    return a + b;
}
func main(): int {
    return add(2, 3);
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = lower_and_verify(&prog).expect("AIR lowering failed");

        let candidates = CandidateGenerator::generate_candidates(&module);
        assert_eq!(candidates.len(), 4);
        assert_eq!(candidates[0].name, "Baseline");
        assert_eq!(candidates[1].name, "EscapePromoted");
        assert_eq!(candidates[2].name, "ConstantFolded");
        assert_eq!(candidates[3].name, "AggressiveCombined");
    }

    #[test]
    fn test_pareto_dominance() {
        let mut candidates = vec![
            (
                "c1".to_string(),
                BenchmarkMetrics {
                    iterations: 5,
                    mean_time_us: 100.0,
                    min_time_us: 90,
                    p95_time_us: 110,
                    stddev_time_us: 5.0,
                    binary_size_bytes: 5000,
                    compile_time_ms: 200,
                    is_pareto_optimal: false,
                },
            ),
            (
                "c2".to_string(), // strictly worse than c1 in all dimensions
                BenchmarkMetrics {
                    iterations: 5,
                    mean_time_us: 150.0,
                    min_time_us: 140,
                    p95_time_us: 160,
                    stddev_time_us: 8.0,
                    binary_size_bytes: 6000,
                    compile_time_ms: 300,
                    is_pareto_optimal: false,
                },
            ),
            (
                "c3".to_string(), // faster than c1, but larger binary
                BenchmarkMetrics {
                    iterations: 5,
                    mean_time_us: 80.0,
                    min_time_us: 75,
                    p95_time_us: 85,
                    stddev_time_us: 3.0,
                    binary_size_bytes: 7000,
                    compile_time_ms: 150,
                    is_pareto_optimal: false,
                },
            ),
        ];

        BenchmarkEvaluator::compute_pareto_frontier(&mut candidates);
        assert!(candidates[0].1.is_pareto_optimal); // c1
        assert!(!candidates[1].1.is_pareto_optimal); // c2 dominated by c1
        assert!(candidates[2].1.is_pareto_optimal); // c3 trade-off (faster, larger)
    }

    #[test]
    fn test_approved_strategy_roundtrip_and_kind_mapping() {
        let temp_dir =
            std::env::temp_dir().join(format!("syntra_approval_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let path =
            ExperimentalCompilationEngine::approve_candidate("cand_1_escape_test", &temp_dir)
                .expect("Approval write failed");
        assert!(path.is_file());

        let loaded =
            ApprovedStrategy::load_from_dir(&temp_dir).expect("Failed to load approved strategy");
        assert_eq!(loaded.approved_candidate_id, "cand_1_escape_test");
        assert_eq!(loaded.status, "APPROVED");
        assert_eq!(loaded.policy, "DeveloperExplicitApproval");
        assert_eq!(
            loaded.to_strategy_kind(),
            candidate::StrategyKind::EscapePromoted
        );

        // Test other mappings
        let cand0 = ApprovedStrategy {
            approved_candidate_id: "cand_0_baseline".to_string(),
            timestamp: 0,
            status: "APPROVED".to_string(),
            policy: "DeveloperExplicitApproval".to_string(),
        };
        assert_eq!(cand0.to_strategy_kind(), candidate::StrategyKind::Baseline);

        let cand2 = ApprovedStrategy {
            approved_candidate_id: "cand_2_constant_folded".to_string(),
            timestamp: 0,
            status: "APPROVED".to_string(),
            policy: "DeveloperExplicitApproval".to_string(),
        };
        assert_eq!(
            cand2.to_strategy_kind(),
            candidate::StrategyKind::ConstantFolded
        );

        let cand3 = ApprovedStrategy {
            approved_candidate_id: "cand_3_aggressive_combined".to_string(),
            timestamp: 0,
            status: "APPROVED".to_string(),
            policy: "DeveloperExplicitApproval".to_string(),
        };
        assert_eq!(
            cand3.to_strategy_kind(),
            candidate::StrategyKind::AggressiveCombined
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_approved_strategy_missing() {
        let nonexistent = Path::new("/tmp/nonexistent_syntra_dir_xyz123");
        assert!(ApprovedStrategy::load_from_dir(nonexistent).is_none());
    }
}
