use syntra_air::ir::AirModule;
use syntra_opt::{ConstantFoldingPass, DeadCodeEliminationPass, EscapePromotionPass};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyKind {
    Baseline,
    EscapePromoted,
    ConstantFolded,
    AggressiveCombined,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub strategy: StrategyKind,
    pub description: String,
    pub air_module: AirModule,
}

pub struct CandidateGenerator;

impl CandidateGenerator {
    pub fn generate_candidates(baseline: &AirModule) -> Vec<Candidate> {
        let mut candidates = Vec::new();

        // 1. Baseline Candidate
        candidates.push(Candidate {
            id: format!("cand_0_{:x}", Self::hash_name("baseline")),
            name: "Baseline".to_string(),
            strategy: StrategyKind::Baseline,
            description: "Standard unoptimized AIR module emitted from compiler frontend"
                .to_string(),
            air_module: baseline.clone(),
        });

        // 2. Escape-Promoted Candidate (Whole-Program Analysis)
        let mut escape_module = baseline.clone();
        let promoted_count = EscapePromotionPass::new().run_on_module(&mut escape_module);
        candidates.push(Candidate {
            id: format!("cand_1_{:x}", Self::hash_name("escape_promoted")),
            name: "EscapePromoted".to_string(),
            strategy: StrategyKind::EscapePromoted,
            description: format!(
                "Promotes non-escaping heap allocations to stack slots (promoted: {promoted_count})"
            ),
            air_module: escape_module,
        });

        // 3. Constant Folded & DCE Candidate
        let mut fold_module = baseline.clone();
        let const_pass = ConstantFoldingPass::new();
        let dce_pass = DeadCodeEliminationPass::new();
        for func in &mut fold_module.functions {
            const_pass.run_on_function(func);
            dce_pass.run_on_function(func);
        }
        candidates.push(Candidate {
            id: format!("cand_2_{:x}", Self::hash_name("constant_folded")),
            name: "ConstantFolded".to_string(),
            strategy: StrategyKind::ConstantFolded,
            description: "Canonical constant propagation, folding, and dead code elimination"
                .to_string(),
            air_module: fold_module,
        });

        // 4. Aggressive Combined Strategy (WPA Escape Promotion + Constant Folding + DCE)
        let mut aggressive_module = baseline.clone();
        EscapePromotionPass::new().run_on_module(&mut aggressive_module);
        for func in &mut aggressive_module.functions {
            const_pass.run_on_function(func);
            dce_pass.run_on_function(func);
        }
        candidates.push(Candidate {
            id: format!("cand_3_{:x}", Self::hash_name("aggressive_combined")),
            name: "AggressiveCombined".to_string(),
            strategy: StrategyKind::AggressiveCombined,
            description:
                "Full optimization pipeline: WPA stack promotion, folding & dead-code elimination"
                    .to_string(),
            air_module: aggressive_module,
        });

        candidates
    }

    fn hash_name(s: &str) -> u32 {
        let mut h: u32 = 0x811c9dc5;
        for b in s.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x01000193);
        }
        h
    }
}
