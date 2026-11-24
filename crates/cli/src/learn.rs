use clap::Subcommand;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use syntra_intelligence::{
    GovernanceAuditLog, ModelEvaluator, ModelPredictor, OptimizationDataset, PromotionGateDecision,
    SupervisedEnsembleModel, VersionedModelRegistry,
};

#[derive(Subcommand, Debug)]
pub enum LearnCommand {
    /// Ingest dataset, validate/quarantine, split without leakage, train, and run promotion gate
    Train {
        /// Path to dataset jsonl file (defaults to .syntra/datasets/training_records.jsonl)
        #[arg(short, long)]
        dataset: Option<PathBuf>,
        /// Ratio of held-out test data for evaluation (default: 0.20)
        #[arg(long, default_value = "0.20")]
        test_ratio: f64,
    },
    /// Evaluate active (or specified) model on held-out test data with comprehensive metrics
    Eval {
        /// Model version to evaluate (defaults to active version)
        #[arg(short, long)]
        version: Option<String>,
        /// Path to evaluation test dataset (defaults to .syntra/datasets/training_records.jsonl)
        #[arg(short, long)]
        dataset: Option<PathBuf>,
    },
    /// List all models in the versioned registry with approval, calibration, and RMSE metrics
    List,
    /// Rollback active model to a previous approved version (LEARN-013)
    Rollback {
        /// Target model version to activate
        version: String,
    },
    /// Revoke a model version due to security or regression flaws (LEARN-013)
    Revoke {
        /// Model version to revoke
        version: String,
        /// Reason for revocation
        #[arg(short, long)]
        reason: String,
    },
    /// Inspect dataset quality, quarantine log, and governance audit trail (LEARN-005)
    Audit,
}

pub fn handle_learn_command(cmd: LearnCommand) {
    let dataset_default = PathBuf::from(".syntra/datasets/training_records.jsonl");
    let quarantine_default = PathBuf::from(".syntra/datasets/quarantine.jsonl");
    let audit_default = PathBuf::from(".syntra/models/audit.log");
    let registry_default = PathBuf::from(".syntra/models/registry.json");

    match cmd {
        LearnCommand::Train {
            dataset,
            test_ratio,
        } => {
            let path = dataset.unwrap_or(dataset_default);
            println!("🧠 Syntra Learning System (SLS Training Pipeline)");
            println!("═════════════════════════════════════════════════════════════");
            println!("📁 Ingesting dataset: {}", path.display());

            if !path.is_file() {
                eprintln!("❌ Dataset file not found: {}", path.display());
                eprintln!("Tip: Run 'syn experiment <file.syn>' or 'syn tune <file.syn>' to generate empirical benchmark records.");
                exit(1);
            }

            let raw_dataset = match OptimizationDataset::load_from_file(&path) {
                Ok(ds) => ds,
                Err(e) => {
                    eprintln!("❌ Failed to load dataset: {e}");
                    exit(1);
                }
            };

            let mut audit_log =
                GovernanceAuditLog::load_from_file(&audit_default).unwrap_or_default();
            let mut validated_dataset = OptimizationDataset::new();

            // 1. Validation & Quarantining (LEARN-002)
            println!(
                "🛡️  Validating records & filtering poisoned/contradictory data (LEARN-002)..."
            );
            let (accepted, quarantined) = validated_dataset.ingest_with_quarantine(
                raw_dataset.records,
                Some(&quarantine_default),
                &mut audit_log,
                Some(&audit_default),
            );
            println!("   Accepted records:    {accepted}");
            if quarantined > 0 {
                println!(
                    "   ⚠️  Quarantined records: {} (logged to {})",
                    quarantined,
                    quarantine_default.display()
                );
            }

            if accepted == 0 {
                eprintln!("❌ No valid records available for training after validation.");
                exit(1);
            }

            // 2. Leakage-Free Train/Test Split (LEARN-003)
            println!("🔒 Splitting into train and held-out test partitions without leakage (LEARN-003)...");
            let (train_set, test_set) = validated_dataset.split_train_test(test_ratio);
            println!("   Training samples:  {}", train_set.records.len());
            println!("   Held-out test:     {}", test_set.records.len());

            // 3. Train Supervised Ensemble Model
            println!("⚡ Training offline supervised ensemble model...");
            let trained_model = match ModelPredictor::train_from_dataset(&train_set) {
                Some(m) => m,
                None => {
                    eprintln!("❌ Insufficient verified records to train candidate model.");
                    exit(1);
                }
            };

            let baseline = SupervisedEnsembleModel::new_default_baseline(
                &trained_model.metadata.target_architecture,
            );

            // 4. Held-Out Evaluation & Promotion Gate (LEARN-011)
            println!("📊 Running held-out evaluation and promotion gate (LEARN-011)...");
            let gate_decision = ModelEvaluator::evaluate_for_promotion(
                &trained_model.ensemble,
                &baseline,
                &test_set,
            );

            match gate_decision {
                PromotionGateDecision::Approved(report) => {
                    println!(
                        "\n✅ PROMOTION GATE PASSED: Model candidate approved for production!"
                    );
                    println!("─────────────────────────────────────────────────────────────");
                    println!("   Model ID:               {}", report.model_id);
                    println!("   Version:                {}", report.version);
                    println!("   Test Samples Evaluated: {}", report.test_samples_count);
                    println!("   Candidate RMSE:         {:.2}%", report.rmse_speedup_pct);
                    println!(
                        "   Baseline RMSE:          {:.2}%",
                        report.baseline_rmse_speedup_pct
                    );
                    println!(
                        "   Relative Improvement:   {:.1}%",
                        report.relative_improvement_pct
                    );
                    println!(
                        "   Spearman Rank Corr:     {:.3}",
                        report.spearman_rank_correlation
                    );
                    println!(
                        "   Calibration Error:      {:.3}",
                        report.expected_calibration_error
                    );

                    let mut registry = VersionedModelRegistry::load_or_default(&registry_default);
                    registry.register_model(trained_model, Some(&audit_default));
                    if let Err(e) = registry.save_registry_manifest(&registry_default) {
                        eprintln!("⚠️  Warning: Failed to save registry manifest: {e}");
                    } else {
                        println!(
                            "💾 Active model registered in {}",
                            registry_default.display()
                        );
                    }
                }
                PromotionGateDecision::Rejected { reason, report } => {
                    println!(
                        "\n⛔ PROMOTION GATE REJECTED: Model candidate failed acceptance criteria!"
                    );
                    println!("─────────────────────────────────────────────────────────────");
                    println!("   Reason:                 {reason}");
                    println!(
                        "   Candidate RMSE:         {:.2}% (Baseline: {:.2}%)",
                        report.rmse_speedup_pct, report.baseline_rmse_speedup_pct
                    );
                    println!(
                        "   Spearman Rank Corr:     {:.3}",
                        report.spearman_rank_correlation
                    );
                    println!(
                        "   Calibration Error:      {:.3}",
                        report.expected_calibration_error
                    );
                    println!("ℹ️  Previous approved model remains active (zero silent production alteration).");
                }
            }
        }
        LearnCommand::Eval { version, dataset } => {
            let path = dataset.unwrap_or(dataset_default);
            println!("📊 Syntra Learning System (Model Evaluation)");
            println!("═════════════════════════════════════════════════════════════");

            if !path.is_file() {
                eprintln!("❌ Evaluation dataset not found: {}", path.display());
                exit(1);
            }

            let ds = OptimizationDataset::load_from_file(&path).unwrap_or_default();
            let registry = VersionedModelRegistry::load_or_default(&registry_default);

            let target_version = version.as_deref().unwrap_or(registry.active_version());
            let model = match registry.get_model(target_version) {
                Some(m) => m,
                None => {
                    eprintln!("❌ Model version '{target_version}' not found in registry.");
                    exit(1);
                }
            };

            let baseline =
                SupervisedEnsembleModel::new_default_baseline(&model.metadata.target_architecture);
            let report = ModelEvaluator::evaluate(&model.ensemble, &baseline, &ds);

            println!(
                "   Target Model:           {} ({})",
                report.model_id, report.version
            );
            println!("   Dataset Evaluated:      {}", path.display());
            println!("   Samples Count:          {}", report.test_samples_count);
            println!("   RMSE Speedup:           {:.2}%", report.rmse_speedup_pct);
            println!("   MAE Speedup:            {:.2}%", report.mae_speedup_pct);
            println!(
                "   Baseline RMSE:          {:.2}%",
                report.baseline_rmse_speedup_pct
            );
            println!(
                "   Relative Improvement:   {:.1}%",
                report.relative_improvement_pct
            );
            println!(
                "   Spearman Rank Corr:     {:.3}",
                report.spearman_rank_correlation
            );
            println!(
                "   Calibration Error:      {:.3}",
                report.expected_calibration_error
            );
            println!(
                "   Gate Status:            {}",
                if report.passed_gate {
                    "APPROVED"
                } else {
                    "REJECTED"
                }
            );
        }
        LearnCommand::List => {
            println!("🧠 Syntra Learning System (Registered Models)");
            println!("══════════════════════════════════════════════════════════════════════════════════");
            println!(
                "{:<24} {:<16} {:<12} {:<10} {:<8} {:<8} {:<8}",
                "MODEL ID", "VERSION", "DIGEST", "ACCURACY", "RMSE", "STATUS", "ACTIVE"
            );
            println!("──────────────────────────────────────────────────────────────────────────────────");

            let registry = VersionedModelRegistry::load_or_default(&registry_default);
            let models = registry.list_models();
            let active = registry.active_version();

            for m in models {
                let status = if m.is_revoked {
                    "REVOKED"
                } else if m.is_approved {
                    "APPROVED"
                } else {
                    "UNAPPROVED"
                };

                let is_active_str = if m.version == active {
                    "★ ACTIVE"
                } else {
                    ""
                };
                let short_digest = if m.digest.len() > 10 {
                    &m.digest[..10]
                } else {
                    &m.digest
                };

                println!(
                    "{:<24} {:<16} {:<12} {:<10.1}% {:<8.2} {:<8} {:<8}",
                    m.model_id,
                    m.version,
                    short_digest,
                    m.evaluation_accuracy * 100.0,
                    m.rmse,
                    status,
                    is_active_str
                );
            }
        }
        LearnCommand::Rollback { version } => {
            println!("⏪ Syntra Learning System (Model Rollback - LEARN-013)");
            println!("═════════════════════════════════════════════════════════════");

            let mut registry = VersionedModelRegistry::load_or_default(&registry_default);
            match registry.rollback_to(&version, Some(&audit_default)) {
                Ok(()) => {
                    let _ = registry.save_registry_manifest(&registry_default);
                    println!("✅ Successfully rolled back active model to version '{version}'");
                    println!("📝 Audit trail logged to {}", audit_default.display());
                }
                Err(err) => {
                    eprintln!("❌ Rollback failed: {err}");
                    exit(1);
                }
            }
        }
        LearnCommand::Revoke { version, reason } => {
            println!("🚫 Syntra Learning System (Model Revocation - LEARN-013)");
            println!("═════════════════════════════════════════════════════════════");

            let mut registry = VersionedModelRegistry::load_or_default(&registry_default);
            match registry.revoke(&version, &reason, Some(&audit_default)) {
                Ok(()) => {
                    let _ = registry.save_registry_manifest(&registry_default);
                    println!("✅ Successfully revoked model version '{version}'");
                    println!("   Reason: {reason}");
                    println!("   Active model reverted to: {}", registry.active_version());
                    println!("📝 Audit trail logged to {}", audit_default.display());
                }
                Err(err) => {
                    eprintln!("❌ Revocation failed: {err}");
                    exit(1);
                }
            }
        }
        LearnCommand::Audit => {
            println!("📋 Syntra Learning System (Governance & Quality Audit - LEARN-005)");
            println!("═════════════════════════════════════════════════════════════════════════");

            if quarantine_default.is_file() {
                let content = fs::read_to_string(&quarantine_default).unwrap_or_default();
                let count = content.lines().count();
                println!(
                    "⚠️  Quarantined Records: {} in {}",
                    count,
                    quarantine_default.display()
                );
            } else {
                println!("✅ Quarantined Records: 0 (No corrupt or poisoned data detected)");
            }

            println!("\n📜 Governance Audit Trail ({}):", audit_default.display());
            println!("─────────────────────────────────────────────────────────────────────────");
            if audit_default.is_file() {
                let audit_log =
                    GovernanceAuditLog::load_from_file(&audit_default).unwrap_or_default();
                if audit_log.entries.is_empty() {
                    println!("   No audit entries recorded yet.");
                } else {
                    for entry in audit_log.entries.iter().rev().take(15) {
                        println!(
                            "   [{}] {:<12} {:<16} {}",
                            entry.timestamp_sec, entry.action, entry.actor, entry.details
                        );
                    }
                }
            } else {
                println!("   No audit log file present yet.");
            }
        }
    }
}
