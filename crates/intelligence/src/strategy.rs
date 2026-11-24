/// Scope of application for an optimization strategy (Specification 04, Section 3)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyScope {
    Expression(String),
    Function(String),
    Module,
    RuntimeRegion(String),
}

/// Lifecycle status of an optimization strategy contract
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyStatus {
    Proposed,
    Verified,
    Measured,
    Approved,
    Applied,
    Revoked,
}

/// A structured optimization strategy contract (Specification 04, Section 3)
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyContract {
    pub id: String,
    pub schema_version: u32,
    pub name: String,
    pub agent_origin: String,
    pub scope: StrategyScope,
    pub preconditions: Vec<String>,
    pub safety_obligations: Vec<String>,
    pub estimated_speedup_pct: f64,
    pub compile_cost_estimate: String,
    pub memory_impact: String,
    pub uncertainty_margin_pct: f64,
    pub conflicts_with: Vec<String>,
    pub status: StrategyStatus,
    pub explanation: String,
}

impl StrategyContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        name: &str,
        agent: &str,
        scope: StrategyScope,
        speedup: f64,
        compile_cost: &str,
        memory: &str,
        explanation: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            schema_version: 1,
            name: name.to_string(),
            agent_origin: agent.to_string(),
            scope,
            preconditions: Vec::new(),
            safety_obligations: Vec::new(),
            estimated_speedup_pct: speedup,
            compile_cost_estimate: compile_cost.to_string(),
            memory_impact: memory.to_string(),
            uncertainty_margin_pct: 5.0,
            conflicts_with: Vec::new(),
            status: StrategyStatus::Proposed,
            explanation: explanation.to_string(),
        }
    }

    pub fn with_precondition(mut self, pre: &str) -> Self {
        self.preconditions.push(pre.to_string());
        self
    }

    pub fn with_safety_obligation(mut self, ob: &str) -> Self {
        self.safety_obligations.push(ob.to_string());
        self
    }

    pub fn with_conflict(mut self, conflict_id: &str) -> Self {
        self.conflicts_with.push(conflict_id.to_string());
        self
    }

    pub fn with_uncertainty(mut self, margin: f64) -> Self {
        self.uncertainty_margin_pct = margin;
        self
    }
}
