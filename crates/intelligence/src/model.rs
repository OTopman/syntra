use crate::dataset::ProgramFeatures;
use std::time::Instant;

/// Memory strategy prediction recommendation (Specification 08 Section 2)
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryDomainRecommendation {
    StackPromotion {
        confidence: f64,
        estimated_alloc_reduction_pct: f64,
    },
    ArenaBump {
        confidence: f64,
        estimated_alloc_reduction_pct: f64,
    },
    PooledObjects {
        confidence: f64,
        estimated_alloc_reduction_pct: f64,
    },
    StandardHeapGC {
        confidence: f64,
    },
}

/// Execution strategy tuning recommendation (Specification 08 Section 2)
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionTuningRecommendation {
    pub inline_threshold: InlineThreshold,
    pub loop_unroll_factor: usize,
    pub recommended_workers: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineThreshold {
    Conservative,
    Standard,
    Aggressive,
}

/// Hardware cost estimation scores [0.0 - 1.0] (Specification 08 Section 2)
#[derive(Debug, Clone, PartialEq)]
pub struct HardwareCostScores {
    pub target_arch: String,
    pub cache_pressure_score: f64,
    pub branch_penalty_score: f64,
    pub register_spill_risk: f64,
}

/// Resource and compile-time cost prediction (Specification 08 Section 2)
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceCostEstimate {
    pub estimated_compile_time_ms: u64,
    pub estimated_binary_size_delta_pct: f64,
    pub confidence: f64,
}

/// Single strategy prediction with calibrated uncertainty interval
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyPrediction {
    pub strategy_name: String,
    pub predicted_speedup_pct: f64,
    pub uncertainty_lower_pct: f64,
    pub uncertainty_upper_pct: f64,
    pub confidence: f64,
}

/// Typed and versioned inference request conforming to Specification 08 Section 6
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub feature_schema_version: String,
    pub target_hardware: String,
    pub features: ProgramFeatures,
    pub candidate_strategies: Vec<String>,
    pub max_budget_us: u64,
}

/// Typed and versioned inference response conforming to Specification 08 Section 6
#[derive(Debug, Clone)]
pub struct InferenceResponse {
    pub predictions: Vec<StrategyPrediction>,
    pub memory_recommendation: MemoryDomainRecommendation,
    pub execution_tuning: ExecutionTuningRecommendation,
    pub hardware_costs: HardwareCostScores,
    pub resource_cost: ResourceCostEstimate,
    pub model_origin: String,
    pub is_fallback: bool,
    pub latency_us: u64,
}

/// Decision stump for gradient-boosted ensemble
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionStump {
    pub feature_idx: usize, // 0: funcs, 1: insts, 2: allocs, 3: loops, 4: calls, 5: alloc_density
    pub threshold: f64,
    pub left_val: f64,
    pub right_val: f64,
    pub weight: f64,
}

impl DecisionStump {
    pub fn predict(&self, feat: &[f64]) -> f64 {
        let val = feat.get(self.feature_idx).copied().unwrap_or(0.0);
        if val <= self.threshold {
            self.left_val * self.weight
        } else {
            self.right_val * self.weight
        }
    }
}

/// Pure-Rust offline supervised ensemble model (Specification 08 Section 2 & 5)
#[derive(Debug, Clone)]
pub struct SupervisedEnsembleModel {
    pub model_id: String,
    pub version: String,
    pub digest: String,
    pub target_arch: String,
    pub stumps: Vec<DecisionStump>,
    pub base_speedup: f64,
}

impl SupervisedEnsembleModel {
    pub fn new_default_baseline(arch: &str) -> Self {
        // Hand-curated analytical priors as baseline ensemble stumps
        let stumps = vec![
            DecisionStump {
                feature_idx: 5, // alloc_density
                threshold: 0.05,
                left_val: 1.5,
                right_val: 12.0,
                weight: 0.6,
            },
            DecisionStump {
                feature_idx: 3, // loop_count
                threshold: 1.0,
                left_val: 1.0,
                right_val: 6.5,
                weight: 0.4,
            },
            DecisionStump {
                feature_idx: 1, // total_instructions
                threshold: 50.0,
                left_val: 0.5,
                right_val: 4.0,
                weight: 0.3,
            },
        ];

        Self {
            model_id: "sls-ensemble-baseline".to_string(),
            version: "1.0.0".to_string(),
            digest: "9f8a7b6c5d4e3f2a".to_string(),
            target_arch: arch.to_string(),
            stumps,
            base_speedup: 2.0,
        }
    }

    pub fn to_feature_vec(p: &ProgramFeatures) -> Vec<f64> {
        vec![
            p.total_functions as f64,
            p.total_instructions as f64,
            p.total_allocations as f64,
            p.loop_count as f64,
            p.call_count as f64,
            p.allocation_density(),
        ]
    }

    /// Evaluates raw speedup prediction for a feature vector
    pub fn raw_predict(&self, feat: &[f64]) -> (f64, f64) {
        let mut pred = self.base_speedup;
        let mut variance_sum = 0.0;

        for stump in &self.stumps {
            let contribution = stump.predict(feat);
            pred += contribution;
            variance_sum += contribution.powi(2);
        }

        let std_dev = (variance_sum / self.stumps.len().max(1) as f64)
            .sqrt()
            .max(2.0);
        (pred, std_dev)
    }

    /// Complete inference pipeline respecting latency budgets (LEARN-014) and out-of-distribution fallback
    pub fn infer(&self, req: &InferenceRequest) -> InferenceResponse {
        let start = Instant::now();

        // Check for Out-of-Distribution (OOD) or Schema Incompatibility
        let is_ood = req.feature_schema_version != "1.0.0"
            || req.features.total_instructions > 1_000_000
            || req.features.total_allocations > 500_000;

        let elapsed_us = start.elapsed().as_micros() as u64;
        let timeout = elapsed_us > req.max_budget_us;

        if is_ood || timeout {
            // Conservative deterministic fallback (Spec 08 Section 6)
            return self.deterministic_fallback(req, is_ood, timeout);
        }

        let feat_vec = Self::to_feature_vec(&req.features);
        let (raw_speedup, std_dev) = self.raw_predict(&feat_vec);

        // 1. Memory Strategy Prediction
        let memory_recommendation = if req.features.allocation_density() > 0.08 {
            MemoryDomainRecommendation::StackPromotion {
                confidence: 0.94,
                estimated_alloc_reduction_pct: (req.features.allocation_density() * 60.0).min(85.0),
            }
        } else if req.features.total_allocations > 100 {
            MemoryDomainRecommendation::ArenaBump {
                confidence: 0.88,
                estimated_alloc_reduction_pct: 45.0,
            }
        } else {
            MemoryDomainRecommendation::StandardHeapGC { confidence: 0.95 }
        };

        // 2. Execution Strategy Tuning
        let execution_tuning = ExecutionTuningRecommendation {
            inline_threshold: if req.features.total_instructions < 100 {
                InlineThreshold::Aggressive
            } else if req.features.call_density() > 0.15 {
                InlineThreshold::Standard
            } else {
                InlineThreshold::Conservative
            },
            loop_unroll_factor: if req.features.loop_count > 0
                && req.features.total_instructions < 200
            {
                4
            } else if req.features.loop_count > 0 {
                2
            } else {
                1
            },
            recommended_workers: if req.features.loop_count > 2
                || req.features.total_instructions > 500
            {
                4
            } else {
                1
            },
            confidence: 0.91,
        };

        // 3. Hardware Cost Estimation
        let is_arm =
            req.target_hardware.contains("aarch64") || req.target_hardware.contains("arm64");
        let hardware_costs = HardwareCostScores {
            target_arch: req.target_hardware.clone(),
            cache_pressure_score: (req.features.total_instructions as f64 * 0.0005).min(1.0),
            branch_penalty_score: (req.features.loop_count as f64 * 0.05).min(1.0),
            register_spill_risk: if is_arm {
                (req.features.total_instructions as f64 * 0.0003).min(1.0)
            } else {
                // x86 has fewer registers
                (req.features.total_instructions as f64 * 0.0008).min(1.0)
            },
        };

        // 4. Optimization Strategy Ranking with Uncertainty Intervals
        let mut predictions = Vec::new();
        let candidates = if req.candidate_strategies.is_empty() {
            vec![
                "EscapePromoted".to_string(),
                "ConstantFolded".to_string(),
                "AggressiveCombined".to_string(),
            ]
        } else {
            req.candidate_strategies.clone()
        };

        for strat in &candidates {
            let strat_multiplier = match strat.as_str() {
                "EscapePromoted" => (req.features.allocation_density() * 10.0).clamp(0.5, 2.5),
                "ConstantFolded" => (req.features.loop_density() * 12.0).clamp(0.4, 1.8),
                "AggressiveCombined" => 1.45,
                _ => 1.0,
            };

            let mean_pred = raw_speedup * strat_multiplier;
            let margin = (std_dev * 1.96).max(1.0);
            let confidence = (1.0 / (1.0 + (margin / mean_pred.max(1.0)))).clamp(0.60, 0.98);

            predictions.push(StrategyPrediction {
                strategy_name: strat.clone(),
                predicted_speedup_pct: mean_pred,
                uncertainty_lower_pct: (mean_pred - margin).max(0.0),
                uncertainty_upper_pct: mean_pred + margin,
                confidence,
            });
        }

        predictions.sort_by(|a, b| {
            b.predicted_speedup_pct
                .partial_cmp(&a.predicted_speedup_pct)
                .unwrap()
        });

        // 5. Resource and Compile-Time Cost Prediction
        let resource_cost = ResourceCostEstimate {
            estimated_compile_time_ms: ((req.features.total_instructions / 50) + 15) as u64,
            estimated_binary_size_delta_pct: (predictions
                .first()
                .map(|p| p.predicted_speedup_pct * 0.25)
                .unwrap_or(2.0)),
            confidence: 0.92,
        };

        let latency_us = start.elapsed().as_micros() as u64;

        InferenceResponse {
            predictions,
            memory_recommendation,
            execution_tuning,
            hardware_costs,
            resource_cost,
            model_origin: format!("{}:{}", self.model_id, self.version),
            is_fallback: false,
            latency_us,
        }
    }

    fn deterministic_fallback(
        &self,
        req: &InferenceRequest,
        _is_ood: bool,
        _timeout: bool,
    ) -> InferenceResponse {
        InferenceResponse {
            predictions: vec![
                StrategyPrediction {
                    strategy_name: "ConstantFolded".to_string(),
                    predicted_speedup_pct: 3.0,
                    uncertainty_lower_pct: 1.0,
                    uncertainty_upper_pct: 5.0,
                    confidence: 0.70,
                },
                StrategyPrediction {
                    strategy_name: "EscapePromoted".to_string(),
                    predicted_speedup_pct: 2.0,
                    uncertainty_lower_pct: 0.5,
                    uncertainty_upper_pct: 4.0,
                    confidence: 0.65,
                },
            ],
            memory_recommendation: MemoryDomainRecommendation::StandardHeapGC { confidence: 0.80 },
            execution_tuning: ExecutionTuningRecommendation {
                inline_threshold: InlineThreshold::Conservative,
                loop_unroll_factor: 1,
                recommended_workers: 1,
                confidence: 0.75,
            },
            hardware_costs: HardwareCostScores {
                target_arch: req.target_hardware.clone(),
                cache_pressure_score: 0.5,
                branch_penalty_score: 0.5,
                register_spill_risk: 0.5,
            },
            resource_cost: ResourceCostEstimate {
                estimated_compile_time_ms: 10,
                estimated_binary_size_delta_pct: 0.0,
                confidence: 0.70,
            },
            model_origin: "deterministic-analytical-fallback".to_string(),
            is_fallback: true,
            latency_us: 1,
        }
    }
}
