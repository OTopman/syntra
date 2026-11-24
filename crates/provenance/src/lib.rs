use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use syntra_air::ir::AirModule;

/// BuildIdentity records cryptographic provenance for Syntra compilation artefacts (Specification 09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildIdentity {
    pub source_digest: String,
    pub air_digest: String,
    pub target_triple: String,
    pub compiler_version: String,
    pub applied_strategies: Vec<String>,
    pub composite_manifest_id: String,
}

impl BuildIdentity {
    pub fn compute(source: &str, module: &AirModule, target: &str, strategies: &[String]) -> Self {
        let mut s_hasher = DefaultHasher::new();
        source.hash(&mut s_hasher);
        let source_digest = format!("{:016x}", s_hasher.finish());

        let mut a_hasher = DefaultHasher::new();
        module.name.hash(&mut a_hasher);
        module.functions.len().hash(&mut a_hasher);
        for func in &module.functions {
            func.name.hash(&mut a_hasher);
            func.blocks.len().hash(&mut a_hasher);
        }
        let air_digest = format!("{:016x}", a_hasher.finish());

        let mut c_hasher = DefaultHasher::new();
        source_digest.hash(&mut c_hasher);
        air_digest.hash(&mut c_hasher);
        target.hash(&mut c_hasher);
        for s in strategies {
            s.hash(&mut c_hasher);
        }
        let composite_manifest_id = format!("{:016x}", c_hasher.finish());

        Self {
            source_digest,
            air_digest,
            target_triple: target.to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            applied_strategies: strategies.to_vec(),
            composite_manifest_id,
        }
    }

    pub fn to_manifest_text(&self) -> String {
        format!(
            "--- Syntra Build Manifest ---\n\
             Manifest ID: {}\n\
             Compiler Version: {}\n\
             Target Architecture: {}\n\
             Source Digest: {}\n\
             AIR Digest: {}\n\
             Applied Strategies: {}\n",
            self.composite_manifest_id,
            self.compiler_version,
            self.target_triple,
            self.source_digest,
            self.air_digest,
            if self.applied_strategies.is_empty() {
                "none (baseline)".to_string()
            } else {
                self.applied_strategies.join(", ")
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_identity_determinism() {
        let module = AirModule {
            name: "test_mod".to_string(),
            functions: Vec::new(),
            class_sizes: std::collections::HashMap::new(),
        };
        let src = "func main(): int { return 42; }";
        let target = "x86_64-apple-darwin";
        let strats = vec!["EscapePromotion".to_string(), "ConstantFolding".to_string()];

        let id1 = BuildIdentity::compute(src, &module, target, &strats);
        let id2 = BuildIdentity::compute(src, &module, target, &strats);

        assert_eq!(id1, id2);
        assert_eq!(id1.source_digest, id2.source_digest);
        assert_eq!(id1.air_digest, id2.air_digest);
        assert_eq!(id1.composite_manifest_id, id2.composite_manifest_id);
    }

    #[test]
    fn test_build_identity_sensitivity_and_manifest_text() {
        let module = AirModule {
            name: "test_mod".to_string(),
            functions: Vec::new(),
            class_sizes: std::collections::HashMap::new(),
        };
        let src1 = "func main(): int { return 42; }";
        let src2 = "func main(): int { return 43; }";
        let target = "x86_64-apple-darwin";
        let strats = vec!["EscapePromotion".to_string()];

        let id1 = BuildIdentity::compute(src1, &module, target, &strats);
        let id2 = BuildIdentity::compute(src2, &module, target, &strats);

        assert_ne!(id1.source_digest, id2.source_digest);
        assert_ne!(id1.composite_manifest_id, id2.composite_manifest_id);

        let manifest = id1.to_manifest_text();
        assert!(manifest.contains("--- Syntra Build Manifest ---"));
        assert!(manifest.contains(&id1.composite_manifest_id));
        assert!(manifest.contains(target));
        assert!(manifest.contains("EscapePromotion"));
    }
}
