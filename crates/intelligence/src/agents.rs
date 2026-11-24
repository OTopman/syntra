use crate::dataset::ProgramFeatures;
use crate::strategy::{StrategyContract, StrategyScope};
use syntra_air::ir::AirModule;
use syntra_analysis::AnalysisReport;
use syntra_runtime::RuntimeProfileData;

/// The unified trait for all specialized logical agents in SIE
pub trait IntelligenceAgent: Send + Sync {
    fn agent_name(&self) -> &'static str;
    fn propose_strategies(
        &self,
        module: &AirModule,
        features: &ProgramFeatures,
        analysis: Option<&AnalysisReport>,
        profile: Option<&RuntimeProfileData>,
    ) -> Vec<StrategyContract>;
}

/// 1. Programme Agent: analyzes control-flow, dead instructions, and invariants
pub struct ProgrammeAgent;

impl IntelligenceAgent for ProgrammeAgent {
    fn agent_name(&self) -> &'static str {
        "ProgrammeAgent"
    }

    fn propose_strategies(
        &self,
        _module: &AirModule,
        features: &ProgramFeatures,
        _analysis: Option<&AnalysisReport>,
        _profile: Option<&RuntimeProfileData>,
    ) -> Vec<StrategyContract> {
        let mut proposals = Vec::new();

        if features.total_instructions > 5 {
            proposals.push(
                StrategyContract::new(
                    "SIE-PROG-DCE",
                    "Canonical Dead Code Elimination",
                    self.agent_name(),
                    StrategyScope::Module,
                    4.5,
                    "Low (O(N) dataflow sweep)",
                    "Neutral",
                    "Eliminates unreachable basic blocks and dead value computations.",
                )
                .with_precondition("Dominator tree and CFG verification verified")
                .with_safety_obligation("No side-effecting operations pruned"),
            );
        }

        if features.total_instructions > 15 {
            proposals.push(
                StrategyContract::new(
                    "SIE-PROG-CONST-FOLD",
                    "Interprocedural Constant Folding",
                    self.agent_name(),
                    StrategyScope::Module,
                    6.0,
                    "Low (Symbolic constant propagation)",
                    "Zero runtime footprint",
                    "Folds compile-time arithmetic operations and simplifies branch conditions.",
                )
                .with_precondition("Constant operand types verified")
                .with_safety_obligation("IEEE-754 arithmetic exactness preserved"),
            );
        }

        proposals
    }
}

/// 2. Memory Agent: analyzes memory domains, stack promotion, and region lifetimes
pub struct MemoryAgent;

impl IntelligenceAgent for MemoryAgent {
    fn agent_name(&self) -> &'static str {
        "MemoryAgent"
    }

    fn propose_strategies(
        &self,
        _module: &AirModule,
        features: &ProgramFeatures,
        analysis: Option<&AnalysisReport>,
        _profile: Option<&RuntimeProfileData>,
    ) -> Vec<StrategyContract> {
        let mut proposals = Vec::new();

        let non_escaping = analysis
            .map(|a| a.escape_report.proven_non_escaping)
            .unwrap_or(0);

        if non_escaping > 0 || features.total_allocations > 0 {
            proposals.push(
                StrategyContract::new(
                    "SIE-MEM-STACK-PROMOTE",
                    "Stack Slot Register Promotion",
                    self.agent_name(),
                    StrategyScope::Module,
                    14.0,
                    "Low (Cranelift SSA register assignment)",
                    "Reduced heap pressure (-100% allocation overhead for proven objects)",
                    "Promotes provably non-escaping allocations to stack registers.",
                )
                .with_precondition("Whole-Program Escape Analysis proves NoEscape")
                .with_safety_obligation("Object lifetime does not exceed caller stack frame")
                .with_uncertainty(2.0),
            );
        }

        let has_arena = analysis
            .map(|a| {
                a.escape_report
                    .facts
                    .iter()
                    .any(|f| f.domain == syntra_air::ir::MemoryDomain::Arena)
            })
            .unwrap_or(false);

        if has_arena {
            proposals.push(
                StrategyContract::new(
                    "SIE-MEM-ARENA-RECYCLE",
                    "Arena Region Fast Coalescing",
                    self.agent_name(),
                    StrategyScope::Module,
                    9.5,
                    "Minimal (O(1) arena pointer reset)",
                    "Zero per-object free overhead",
                    "Reclaims entire memory arenas in a single operation upon scope exit.",
                )
                .with_precondition("[MEM-023] Arena lifetime invariants hold")
                .with_safety_obligation("No active references outlive the region boundary"),
            );
        }

        proposals
    }
}

/// 3. Execution Agent: evaluates parallelism, scheduling, and concurrency
pub struct ExecutionAgent;

impl IntelligenceAgent for ExecutionAgent {
    fn agent_name(&self) -> &'static str {
        "ExecutionAgent"
    }

    fn propose_strategies(
        &self,
        _module: &AirModule,
        features: &ProgramFeatures,
        analysis: Option<&AnalysisReport>,
        _profile: Option<&RuntimeProfileData>,
    ) -> Vec<StrategyContract> {
        let mut proposals = Vec::new();

        let has_concurrency = analysis
            .map(|a| !a.effect_report.concurrent_functions.is_empty())
            .unwrap_or(false);

        if has_concurrency || features.total_functions >= 3 {
            proposals.push(
                StrategyContract::new(
                    "SIE-EXEC-WORK-STEALING",
                    "Adaptive Work-Stealing Task Scheduler",
                    self.agent_name(),
                    StrategyScope::Module,
                    18.0,
                    "Medium (Task scheduler runtime hook)",
                    "Dynamic worker pool (1 to 2x CPU cores)",
                    "Distributes concurrent tasks across CPU cores with lock-free dequeues.",
                )
                .with_precondition("Function effect set verifies thread safety")
                .with_safety_obligation("No unshared mutable state across task boundaries"),
            );

            proposals.push(
                StrategyContract::new(
                    "SIE-EXEC-CHANNEL-AMORT",
                    "Channel Message Batching & Amortization",
                    self.agent_name(),
                    StrategyScope::Module,
                    12.5,
                    "Low (Ring-buffer channel optimization)",
                    "Bounded buffer footprint",
                    "Amortizes synchronization costs across high-throughput channel transfers.",
                )
                .with_precondition("Channel message volume meets throughput threshold")
                .with_safety_obligation("Message order strictly preserved"),
            );
        }

        proposals
    }
}

/// 4. Hardware Agent: evaluates target ISA, vectorization, and cache alignment
pub struct HardwareAgent;

impl IntelligenceAgent for HardwareAgent {
    fn agent_name(&self) -> &'static str {
        "HardwareAgent"
    }

    fn propose_strategies(
        &self,
        _module: &AirModule,
        _features: &ProgramFeatures,
        _analysis: Option<&AnalysisReport>,
        _profile: Option<&RuntimeProfileData>,
    ) -> Vec<StrategyContract> {
        vec![StrategyContract::new(
            "SIE-HW-ISA-SELECT",
            "Target Architecture Instruction Selection & PIC",
            self.agent_name(),
            StrategyScope::Module,
            8.0,
            "Standard (Cranelift native codegen)",
            "Optimal 64-byte L1 cache-line alignment",
            "Encodes native instructions matching host hardware ISA (ARM64 / x86_64).",
        )
        .with_precondition("Host target ISA supported by Cranelift backend")
        .with_safety_obligation("Position-independent code invariants verified")]
    }
}

/// 5. Performance Agent: interprets profile telemetry and identifies hot paths
pub struct PerformanceAgent;

impl IntelligenceAgent for PerformanceAgent {
    fn agent_name(&self) -> &'static str {
        "PerformanceAgent"
    }

    fn propose_strategies(
        &self,
        _module: &AirModule,
        _features: &ProgramFeatures,
        _analysis: Option<&AnalysisReport>,
        profile: Option<&RuntimeProfileData>,
    ) -> Vec<StrategyContract> {
        let mut proposals = Vec::new();

        if let Some(prof) = profile {
            for (func_name, fp) in &prof.functions {
                if prof.is_hot(func_name) {
                    proposals.push(
                        StrategyContract::new(
                            &format!("SIE-PERF-INLINE-{func_name}"),
                            &format!("Hot Path Inlining for {func_name}()"),
                            self.agent_name(),
                            StrategyScope::Function(func_name.clone()),
                            22.5,
                            "Moderate (Inline expansion & code duplication)",
                            "Small code size expansion (< 15%)",
                            &format!(
                                "Telemetry recorded {} calls ({:.1} µs). Inlining eliminates call-frame overhead on critical hot path.",
                                fp.call_count, fp.total_duration.as_micros() as f64
                            )
                        )
                        .with_precondition("Function is marked hot in profile lockfile")
                        .with_safety_obligation("Recursion depth bounded to avoid code explosion")
                        .with_conflict("SIE-RES-TAIL-CALL")
                    );
                }
            }
        }

        proposals
    }
}

/// 6. Research Agent: proposes advanced pattern transformations from known catalogs
pub struct ResearchAgent;

impl IntelligenceAgent for ResearchAgent {
    fn agent_name(&self) -> &'static str {
        "ResearchAgent"
    }

    fn propose_strategies(
        &self,
        module: &AirModule,
        _features: &ProgramFeatures,
        _analysis: Option<&AnalysisReport>,
        _profile: Option<&RuntimeProfileData>,
    ) -> Vec<StrategyContract> {
        let mut proposals = Vec::new();

        for func in &module.functions {
            // Check for recursion
            let is_recursive = func.blocks.iter().any(|b| {
                b.instructions.iter().any(|inst| {
                    if let syntra_air::ir::Op::Call { func_name, .. } = &inst.op {
                        func_name == &func.name
                    } else {
                        false
                    }
                })
            });

            if is_recursive {
                proposals.push(
                    StrategyContract::new(
                        &format!("SIE-RES-TAIL-CALL-{}", func.name),
                        &format!("Tail Recursion Elimination for {}()", func.name),
                        self.agent_name(),
                        StrategyScope::Function(func.name.clone()),
                        16.0,
                        "Low (Transform tail call into loop jump)",
                        "Constant stack space O(1)",
                        "Replaces recursive call frame allocation with in-place loop branch.",
                    )
                    .with_precondition("Self-call is in tail position before return")
                    .with_safety_obligation("Preserves parameter evaluation semantics")
                    .with_conflict(&format!("SIE-PERF-INLINE-{}", func.name)),
                );
            }
        }

        proposals
    }
}
