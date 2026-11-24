use crate::governance::{GovernanceAuditEntry, GovernanceAuditLog, QuarantineReason};
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;

/// Precise failure label for optimization candidates conforming to Specification 08 Section 3
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureReason {
    InvalidPrecondition,
    VerifierFailure,
    SemanticMismatch,
    CompileBudgetExceeded,
    PoorPerformance,
    MemoryBudgetExceeded,
    TargetIncompatibility,
    Custom(String),
}

impl FailureReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::InvalidPrecondition => "InvalidPrecondition",
            Self::VerifierFailure => "VerifierFailure",
            Self::SemanticMismatch => "SemanticMismatch",
            Self::CompileBudgetExceeded => "CompileBudgetExceeded",
            Self::PoorPerformance => "PoorPerformance",
            Self::MemoryBudgetExceeded => "MemoryBudgetExceeded",
            Self::TargetIncompatibility => "TargetIncompatibility",
            Self::Custom(s) => s.as_str(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "InvalidPrecondition" => Self::InvalidPrecondition,
            "VerifierFailure" => Self::VerifierFailure,
            "SemanticMismatch" => Self::SemanticMismatch,
            "CompileBudgetExceeded" => Self::CompileBudgetExceeded,
            "PoorPerformance" => Self::PoorPerformance,
            "MemoryBudgetExceeded" => Self::MemoryBudgetExceeded,
            "TargetIncompatibility" => Self::TargetIncompatibility,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for FailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Extracted structural and syntactic features from AIR
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramFeatures {
    pub total_functions: usize,
    pub total_instructions: usize,
    pub total_allocations: usize,
    pub loop_count: usize,
    pub call_count: usize,
}

impl ProgramFeatures {
    pub fn extract_from_air(module: &syntra_air::ir::AirModule) -> Self {
        let total_functions = module.functions.len();
        let mut total_instructions = 0;
        let mut total_allocations = 0;
        let mut call_count = 0;
        let mut loop_count = 0;

        for func in &module.functions {
            for (idx, block) in func.blocks.iter().enumerate() {
                total_instructions += block.instructions.len();
                for inst in &block.instructions {
                    if matches!(
                        inst.op,
                        syntra_air::Op::Alloc { .. }
                            | syntra_air::Op::NewObject { .. }
                            | syntra_air::Op::AllocArray { .. }
                    ) {
                        total_allocations += 1;
                    }
                    if matches!(inst.op, syntra_air::Op::Call { .. }) {
                        call_count += 1;
                    }
                }
                match &block.terminator {
                    syntra_air::Terminator::Branch(target, _, _) => {
                        if target.0 <= idx {
                            loop_count += 1;
                        }
                    }
                    syntra_air::Terminator::CondBranch {
                        then_block,
                        else_block,
                        ..
                    } => {
                        if then_block.0 <= idx || else_block.0 <= idx {
                            loop_count += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        Self {
            total_functions,
            total_instructions,
            total_allocations,
            loop_count,
            call_count,
        }
    }

    pub fn allocation_density(&self) -> f64 {
        if self.total_instructions > 0 {
            self.total_allocations as f64 / self.total_instructions as f64
        } else {
            0.0
        }
    }

    pub fn call_density(&self) -> f64 {
        if self.total_instructions > 0 {
            self.call_count as f64 / self.total_instructions as f64
        } else {
            0.0
        }
    }

    pub fn loop_density(&self) -> f64 {
        if self.total_instructions > 0 {
            self.loop_count as f64 / self.total_instructions as f64
        } else {
            0.0
        }
    }
}

/// Generates a privacy-safe non-reversible identifier (Specification 08, LEARN-004)
pub fn privacy_safe_identifier(raw: &str) -> String {
    let mut hasher = DefaultHasher::new();
    raw.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// A structured training record conforming strictly to Specification 08 Section 3
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingRecord {
    pub feature_schema_version: String,
    pub experiment_id: String,
    pub candidate_digest: String,
    pub program_digest: String,
    pub workload_id: String,
    pub features: ProgramFeatures,
    pub target_os: String,
    pub target_arch: String,
    pub candidate_id: String,
    pub strategy_name: String,
    pub correctness_passed: bool,
    pub compile_time_ms: u64,
    pub mean_latency_us: f64,
    pub speedup_factor: f64,
    pub binary_size_bytes: u64,
    pub sample_count: usize,
    pub measurement_uncertainty: f64,
    pub is_pareto_optimal: bool,
    pub failure_reason: Option<FailureReason>,
}

impl TrainingRecord {
    /// Validates the training record for schema integrity, poison detection, and logical consistency (LEARN-002)
    pub fn validate(&self) -> Result<(), QuarantineReason> {
        if self.feature_schema_version != "1.0.0" {
            return Err(QuarantineReason::SchemaMismatch(
                self.feature_schema_version.clone(),
            ));
        }

        if self.experiment_id.is_empty() {
            return Err(QuarantineReason::MissingMetadata(
                "experiment_id is empty".to_string(),
            ));
        }

        if self.candidate_digest.is_empty() {
            return Err(QuarantineReason::MissingMetadata(
                "candidate_digest is empty".to_string(),
            ));
        }

        if self.mean_latency_us <= 0.0
            || self.mean_latency_us.is_nan()
            || self.mean_latency_us.is_infinite()
        {
            return Err(QuarantineReason::NonPositiveLatency);
        }

        if self.speedup_factor <= 0.0 || self.speedup_factor > 250.0 || self.speedup_factor.is_nan()
        {
            return Err(QuarantineReason::ExtremeAnomalySpeedup(format!(
                "{:.2}x",
                self.speedup_factor
            )));
        }

        // Check for verifier / semantic contradictions
        if self.correctness_passed {
            if let Some(fail) = &self.failure_reason {
                if matches!(
                    fail,
                    FailureReason::VerifierFailure | FailureReason::SemanticMismatch
                ) {
                    return Err(QuarantineReason::ContradictoryVerifierState(format!(
                        "correctness is marked passed but failure label is {fail:?}"
                    )));
                }
            }
        } else if self.failure_reason.is_none() {
            return Err(QuarantineReason::PoisonedData(
                "Failed candidate lacks failure_reason label".to_string(),
            ));
        }

        Ok(())
    }

    pub fn to_json_line(&self) -> String {
        let fail_str = self
            .failure_reason
            .as_ref()
            .map(|r| format!("\"{}\"", r.as_str()))
            .unwrap_or_else(|| "null".to_string());

        format!(
            r#"{{"schema":"{}","exp_id":"{}","cand_digest":"{}","digest":"{}","workload":"{}","funcs":{},"insts":{},"allocs":{},"loops":{},"calls":{},"os":"{}","arch":"{}","cand":"{}","strat":"{}","valid":{},"compile_ms":{},"latency_us":{:.2},"speedup":{:.2},"size":{},"samples":{},"uncertainty":{:.4},"pareto":{},"fail":{}}}"#,
            self.feature_schema_version,
            self.experiment_id,
            self.candidate_digest,
            self.program_digest,
            self.workload_id,
            self.features.total_functions,
            self.features.total_instructions,
            self.features.total_allocations,
            self.features.loop_count,
            self.features.call_count,
            self.target_os,
            self.target_arch,
            self.candidate_id,
            self.strategy_name,
            self.correctness_passed,
            self.compile_time_ms,
            self.mean_latency_us,
            self.speedup_factor,
            self.binary_size_bytes,
            self.sample_count,
            self.measurement_uncertainty,
            self.is_pareto_optimal,
            fail_str
        )
    }

    pub fn from_json_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            return None;
        }

        let extract_field = |key: &str| -> Option<String> {
            let pattern = format!("\"{key}\":");
            let pos = trimmed.find(&pattern)?;
            let rest = &trimmed[pos + pattern.len()..];
            let rest_trimmed = rest.trim_start();
            if let Some(stripped) = rest_trimmed.strip_prefix('"') {
                let closing = stripped.find('"')?;
                Some(stripped[..closing].to_string())
            } else {
                let end = rest_trimmed.find([',', '}']).unwrap_or(rest_trimmed.len());
                Some(rest_trimmed[..end].trim().to_string())
            }
        };

        let feature_schema_version = extract_field("schema")?;
        let program_digest = extract_field("digest").unwrap_or_default();
        let experiment_id =
            extract_field("exp_id").unwrap_or_else(|| format!("exp_{program_digest}"));
        let candidate_digest =
            extract_field("cand_digest").unwrap_or_else(|| format!("cd_{program_digest}"));
        let workload_id = extract_field("workload").unwrap_or_else(|| program_digest.clone());

        let total_functions = extract_field("funcs")?.parse().ok()?;
        let total_instructions = extract_field("insts")?.parse().ok()?;
        let total_allocations = extract_field("allocs")?.parse().ok()?;
        let loop_count = extract_field("loops")?.parse().ok()?;
        let call_count = extract_field("calls")?.parse().ok()?;
        let target_os = extract_field("os").unwrap_or_default();
        let target_arch = extract_field("arch").unwrap_or_default();
        let candidate_id = extract_field("cand")?;
        let strategy_name = extract_field("strat")?;
        let correctness_passed = extract_field("valid")?.parse().unwrap_or(true);
        let compile_time_ms = extract_field("compile_ms")?.parse().unwrap_or(0);
        let mean_latency_us = extract_field("latency_us")?.parse().unwrap_or(0.0);
        let speedup_factor = extract_field("speedup")?.parse().unwrap_or(1.0);
        let binary_size_bytes = extract_field("size")?.parse().unwrap_or(0);
        let sample_count = extract_field("samples")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let measurement_uncertainty = extract_field("uncertainty")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.02);
        let is_pareto_optimal = extract_field("pareto")?.parse().unwrap_or(false);

        let fail_str = extract_field("fail");
        let failure_reason = if fail_str.as_deref() == Some("null") || fail_str.is_none() {
            None
        } else {
            fail_str.map(|s| FailureReason::from_str(&s))
        };

        Some(Self {
            feature_schema_version,
            experiment_id,
            candidate_digest,
            program_digest,
            workload_id,
            features: ProgramFeatures {
                total_functions,
                total_instructions,
                total_allocations,
                loop_count,
                call_count,
            },
            target_os,
            target_arch,
            candidate_id,
            strategy_name,
            correctness_passed,
            compile_time_ms,
            mean_latency_us,
            speedup_factor,
            binary_size_bytes,
            sample_count,
            measurement_uncertainty,
            is_pareto_optimal,
            failure_reason,
        })
    }
}

/// Manages training and benchmark dataset ingestion, quarantining, and leakage-free splitting
#[derive(Debug, Default)]
pub struct OptimizationDataset {
    pub records: Vec<TrainingRecord>,
}

impl OptimizationDataset {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn append(&mut self, record: TrainingRecord) {
        self.records.push(record);
    }

    pub fn append_to_file(&self, record: &TrainingRecord, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("Failed to open dataset file {}: {}", path.display(), e))?;

        writeln!(file, "{}", record.to_json_line())
            .map_err(|e| format!("Failed to write to dataset {}: {}", path.display(), e))
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read dataset file {}: {}", path.display(), e))?;

        let mut records = Vec::new();
        for line in content.lines() {
            if let Some(rec) = TrainingRecord::from_json_line(line) {
                records.push(rec);
            }
        }
        Ok(Self { records })
    }

    /// Ingests records with validation, immediately segregating invalid/poisoned records to quarantine (LEARN-002)
    pub fn ingest_with_quarantine(
        &mut self,
        incoming: Vec<TrainingRecord>,
        quarantine_path: Option<&Path>,
        audit_log: &mut GovernanceAuditLog,
        audit_path: Option<&Path>,
    ) -> (usize, usize) {
        let mut accepted = 0;
        let mut quarantined = 0;

        for record in incoming {
            match record.validate() {
                Ok(()) => {
                    self.records.push(record);
                    accepted += 1;
                }
                Err(reason) => {
                    quarantined += 1;
                    if let Some(qp) = quarantine_path {
                        if let Some(parent) = qp.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(qp) {
                            let _ = writeln!(
                                f,
                                r#"{{"reason":"{}","record":{}}}"#,
                                reason,
                                record.to_json_line()
                            );
                        }
                    }
                    audit_log.record(
                        GovernanceAuditEntry::new(
                            "quarantine",
                            "dataset_validator",
                            &record.candidate_digest,
                            &format!("Quarantined: {reason}"),
                        ),
                        audit_path,
                    );
                }
            }
        }

        audit_log.record(
            GovernanceAuditEntry::new(
                "ingest",
                "dataset_pipeline",
                &format!("batch_{}", accepted + quarantined),
                &format!("Ingested {accepted} accepted records, {quarantined} quarantined"),
            ),
            audit_path,
        );

        (accepted, quarantined)
    }

    /// Splits records into training and test sets strictly by workload/program identity to prevent train/test leakage (LEARN-003)
    pub fn split_train_test(&self, test_ratio: f64) -> (OptimizationDataset, OptimizationDataset) {
        let mut workload_groups: HashMap<String, Vec<TrainingRecord>> = HashMap::new();
        for rec in &self.records {
            workload_groups
                .entry(rec.workload_id.clone())
                .or_default()
                .push(rec.clone());
        }

        let mut workloads: Vec<String> = workload_groups.keys().cloned().collect();
        workloads.sort(); // Deterministic ordering

        let test_count =
            ((workloads.len() as f64 * test_ratio.clamp(0.1, 0.9)).round() as usize).max(1);
        let (test_keys, train_keys) = if workloads.len() > 1 {
            let split_idx = workloads.len().saturating_sub(test_count);
            (&workloads[split_idx..], &workloads[..split_idx])
        } else {
            (&workloads[..], &workloads[..])
        };

        let test_set: HashSet<String> = test_keys.iter().cloned().collect();
        let train_set: HashSet<String> = train_keys.iter().cloned().collect();

        let mut train_records = Vec::new();
        let mut test_records = Vec::new();

        for (w_id, recs) in workload_groups {
            if test_set.contains(&w_id) && !train_set.contains(&w_id) {
                test_records.extend(recs);
            } else {
                train_records.extend(recs);
            }
        }

        (
            OptimizationDataset {
                records: train_records,
            },
            OptimizationDataset {
                records: test_records,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_record_roundtrip() {
        let rec = TrainingRecord {
            feature_schema_version: "1.0.0".to_string(),
            experiment_id: "exp_test_01".to_string(),
            candidate_digest: "cand_digest_99".to_string(),
            program_digest: "digest123".to_string(),
            workload_id: "workload_alpha".to_string(),
            features: ProgramFeatures {
                total_functions: 5,
                total_instructions: 120,
                total_allocations: 15,
                loop_count: 2,
                call_count: 8,
            },
            target_os: "macos".to_string(),
            target_arch: "x86_64".to_string(),
            candidate_id: "cand_1".to_string(),
            strategy_name: "EscapePromoted".to_string(),
            correctness_passed: true,
            compile_time_ms: 45,
            mean_latency_us: 123.45,
            speedup_factor: 1.25,
            binary_size_bytes: 40960,
            sample_count: 10,
            measurement_uncertainty: 0.015,
            is_pareto_optimal: true,
            failure_reason: None,
        };

        assert!(rec.validate().is_ok());

        let json_line = rec.to_json_line();
        let parsed = TrainingRecord::from_json_line(&json_line).expect("failed to parse json line");
        assert_eq!(parsed.feature_schema_version, "1.0.0");
        assert_eq!(parsed.experiment_id, "exp_test_01");
        assert_eq!(parsed.candidate_digest, "cand_digest_99");
        assert_eq!(parsed.program_digest, "digest123");
        assert_eq!(parsed.workload_id, "workload_alpha");
        assert_eq!(parsed.features.total_functions, 5);
        assert_eq!(parsed.sample_count, 10);
        assert_eq!(parsed.speedup_factor, 1.25);
    }

    #[test]
    fn test_poison_data_quarantine() {
        let mut audit_log = GovernanceAuditLog::new();
        let mut dataset = OptimizationDataset::new();

        let valid_rec = TrainingRecord {
            feature_schema_version: "1.0.0".to_string(),
            experiment_id: "exp_01".to_string(),
            candidate_digest: "cd_01".to_string(),
            program_digest: "prog_01".to_string(),
            workload_id: "w_01".to_string(),
            features: ProgramFeatures {
                total_functions: 1,
                total_instructions: 10,
                total_allocations: 0,
                loop_count: 0,
                call_count: 0,
            },
            target_os: "macos".to_string(),
            target_arch: "x86_64".to_string(),
            candidate_id: "c1".to_string(),
            strategy_name: "ConstFold".to_string(),
            correctness_passed: true,
            compile_time_ms: 10,
            mean_latency_us: 50.0,
            speedup_factor: 1.05,
            binary_size_bytes: 1000,
            sample_count: 5,
            measurement_uncertainty: 0.01,
            is_pareto_optimal: true,
            failure_reason: None,
        };

        let mut poisoned_rec = valid_rec.clone();
        poisoned_rec.speedup_factor = 999.0; // Extreme anomaly

        let mut contradictory_rec = valid_rec.clone();
        contradictory_rec.correctness_passed = true;
        contradictory_rec.failure_reason = Some(FailureReason::VerifierFailure); // Contradiction

        let (acc, quar) = dataset.ingest_with_quarantine(
            vec![valid_rec, poisoned_rec, contradictory_rec],
            None,
            &mut audit_log,
            None,
        );

        assert_eq!(acc, 1);
        assert_eq!(quar, 2);
        assert_eq!(dataset.records.len(), 1);
    }

    #[test]
    fn test_leakage_free_split() {
        let mut dataset = OptimizationDataset::new();
        for i in 0..10 {
            let workload = format!("workload_{}", i / 2); // 5 distinct workloads
            dataset.append(TrainingRecord {
                feature_schema_version: "1.0.0".to_string(),
                experiment_id: format!("exp_{i}"),
                candidate_digest: format!("cd_{i}"),
                program_digest: format!("prog_{i}"),
                workload_id: workload,
                features: ProgramFeatures {
                    total_functions: 1,
                    total_instructions: 10,
                    total_allocations: 0,
                    loop_count: 0,
                    call_count: 0,
                },
                target_os: "macos".to_string(),
                target_arch: "x86_64".to_string(),
                candidate_id: format!("c_{i}"),
                strategy_name: "ConstFold".to_string(),
                correctness_passed: true,
                compile_time_ms: 10,
                mean_latency_us: 50.0,
                speedup_factor: 1.05,
                binary_size_bytes: 1000,
                sample_count: 5,
                measurement_uncertainty: 0.01,
                is_pareto_optimal: true,
                failure_reason: None,
            });
        }

        let (train, test) = dataset.split_train_test(0.3);
        let train_workloads: HashSet<_> = train.records.iter().map(|r| &r.workload_id).collect();
        let test_workloads: HashSet<_> = test.records.iter().map(|r| &r.workload_id).collect();

        // Strict zero-intersection guarantee
        for tw in test_workloads {
            assert!(
                !train_workloads.contains(tw),
                "Leakage detected: {tw} exists in both train and test!"
            );
        }
    }
}
