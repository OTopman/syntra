pub mod const_fold;
pub mod dce;
pub mod escape_promoter;
pub mod pgo;

pub use const_fold::ConstantFoldingPass;
pub use dce::DeadCodeEliminationPass;
pub use escape_promoter::EscapePromotionPass;
pub use pgo::PgoOptimizationPass;
use syntra_air::ir::AirModule;
use syntra_air::AirVerifier;
use syntra_diagnostics::Diagnostic;

pub trait OptimizationPass {
    fn name(&self) -> &'static str;
    fn run(&self, module: &mut AirModule) -> Result<bool, Vec<Diagnostic>>;
}

pub struct OptimizationPipeline {
    passes: Vec<Box<dyn OptimizationPass>>,
}

struct ConstantFoldingWrapper(ConstantFoldingPass);
impl OptimizationPass for ConstantFoldingWrapper {
    fn name(&self) -> &'static str {
        "ConstantFolding"
    }
    fn run(&self, module: &mut AirModule) -> Result<bool, Vec<Diagnostic>> {
        let mut changed = false;
        for func in &mut module.functions {
            if self.0.run_on_function(func) {
                changed = true;
            }
        }
        Ok(changed)
    }
}

struct DeadCodeEliminationWrapper(DeadCodeEliminationPass);
impl OptimizationPass for DeadCodeEliminationWrapper {
    fn name(&self) -> &'static str {
        "DeadCodeElimination"
    }
    fn run(&self, module: &mut AirModule) -> Result<bool, Vec<Diagnostic>> {
        let mut changed = false;
        for func in &mut module.functions {
            if self.0.run_on_function(func) {
                changed = true;
            }
        }
        Ok(changed)
    }
}

struct EscapePromotionWrapper(EscapePromotionPass);
impl OptimizationPass for EscapePromotionWrapper {
    fn name(&self) -> &'static str {
        "EscapePromotion"
    }
    fn run(&self, module: &mut AirModule) -> Result<bool, Vec<Diagnostic>> {
        let promoted = self.0.run_on_module(module);
        Ok(promoted > 0)
    }
}

impl OptimizationPipeline {
    pub fn default_pipeline() -> Self {
        Self {
            passes: vec![
                Box::new(ConstantFoldingWrapper(ConstantFoldingPass::new())),
                Box::new(DeadCodeEliminationWrapper(DeadCodeEliminationPass::new())),
                Box::new(EscapePromotionWrapper(EscapePromotionPass::new())),
            ],
        }
    }

    pub fn empty() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn escape_promotion_only() -> Self {
        Self {
            passes: vec![Box::new(EscapePromotionWrapper(EscapePromotionPass::new()))],
        }
    }

    pub fn constant_folding_dce() -> Self {
        Self {
            passes: vec![
                Box::new(ConstantFoldingWrapper(ConstantFoldingPass::new())),
                Box::new(DeadCodeEliminationWrapper(DeadCodeEliminationPass::new())),
            ],
        }
    }

    pub fn aggressive_combined() -> Self {
        Self {
            passes: vec![
                Box::new(EscapePromotionWrapper(EscapePromotionPass::new())),
                Box::new(ConstantFoldingWrapper(ConstantFoldingPass::new())),
                Box::new(DeadCodeEliminationWrapper(DeadCodeEliminationPass::new())),
            ],
        }
    }

    pub fn with_pgo(pgo_pass: PgoOptimizationPass) -> Self {
        Self {
            passes: vec![
                Box::new(ConstantFoldingWrapper(ConstantFoldingPass::new())),
                Box::new(pgo_pass),
                Box::new(DeadCodeEliminationWrapper(DeadCodeEliminationPass::new())),
                Box::new(EscapePromotionWrapper(EscapePromotionPass::new())),
            ],
        }
    }

    /// Run the optimization passes. Enforces invariant verification after every mutating pass!
    pub fn optimize(&self, module: &mut AirModule) -> Result<usize, Vec<Diagnostic>> {
        let mut passes_applied = 0;

        for pass in &self.passes {
            let changed = pass.run(module)?;
            if changed {
                passes_applied += 1;
                // Specification 03: Deterministic verifier invariant check after every pass
                for func in &module.functions {
                    AirVerifier::new(func).verify()?;
                }
            }
        }

        Ok(passes_applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntra_air::ir::*;
    use syntra_diagnostics::Span;
    use syntra_frontend::Type;

    #[test]
    fn test_optimization_pipeline_constructors() {
        let empty = OptimizationPipeline::empty();
        assert_eq!(empty.passes.len(), 0);

        let esc = OptimizationPipeline::escape_promotion_only();
        assert_eq!(esc.passes.len(), 1);
        assert_eq!(esc.passes[0].name(), "EscapePromotion");

        let cfd = OptimizationPipeline::constant_folding_dce();
        assert_eq!(cfd.passes.len(), 2);
        assert_eq!(cfd.passes[0].name(), "ConstantFolding");
        assert_eq!(cfd.passes[1].name(), "DeadCodeElimination");

        let agg = OptimizationPipeline::aggressive_combined();
        assert_eq!(agg.passes.len(), 3);

        let def = OptimizationPipeline::default_pipeline();
        assert_eq!(def.passes.len(), 3);
    }

    #[test]
    fn test_optimize_clean_air_module() {
        let span = Span::new(0, 10);
        let mut module = AirModule {
            name: "test_opt_module".to_string(),
            functions: vec![AirFunction {
                name: "main".to_string(),
                params: vec![],
                return_type: Type::Int,
                blocks: vec![BasicBlock {
                    id: BlockId(0),
                    params: vec![],
                    instructions: vec![Instruction {
                        result: ValueId(0),
                        ty: Type::Int,
                        op: Op::ConstInt(42),
                        span,
                    }],
                    terminator: Terminator::Return(Some(ValueId(0)), span),
                }],
                entry_block: BlockId(0),
            }],
            class_sizes: std::collections::HashMap::new(),
        };

        let pipeline = OptimizationPipeline::default_pipeline();
        let res = pipeline.optimize(&mut module);
        assert!(res.is_ok(), "Optimizing valid module must succeed");
    }
}
