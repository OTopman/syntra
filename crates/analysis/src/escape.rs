use std::collections::{HashMap, HashSet};
use syntra_air::ir::{AirFunction, AirModule, MemoryDomain, Op, Terminator, ValueId};
use syntra_frontend::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofStatus {
    Proven,
    Checked,
    Observed,
    Predicted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapeState {
    NoEscape,
    Escaped(EscapeReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapeReason {
    Returned,
    StoredToHeap(String),
    StoredToArray,
    PassedToCall(String),
    ThreadEscape,
}

#[derive(Debug, Clone)]
pub struct AllocationFact {
    pub value_id: ValueId,
    pub function_name: String,
    pub ty: Type,
    pub domain: MemoryDomain,
    pub status: ProofStatus,
    pub escape_state: EscapeState,
    pub promotable_to_stack: bool,
    pub violates_arena_lifetime: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EscapeReport {
    pub facts: Vec<AllocationFact>,
    pub total_allocations: usize,
    pub proven_non_escaping: usize,
    pub escaped_allocations: usize,
    pub arena_violations: usize,
}

impl EscapeReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn promotable_count(&self) -> usize {
        self.facts.iter().filter(|f| f.promotable_to_stack).count()
    }
}

pub struct EscapeAnalyzer;

impl EscapeAnalyzer {
    pub fn analyze_module(module: &AirModule) -> EscapeReport {
        let mut report = EscapeReport::new();

        for func in &module.functions {
            let func_facts = Self::analyze_function(func, module);
            for fact in func_facts {
                report.total_allocations += 1;
                if fact.promotable_to_stack {
                    report.proven_non_escaping += 1;
                } else {
                    report.escaped_allocations += 1;
                }
                if fact.violates_arena_lifetime {
                    report.arena_violations += 1;
                }
                report.facts.push(fact);
            }
        }

        report
    }

    pub fn analyze_function(func: &AirFunction, _module: &AirModule) -> Vec<AllocationFact> {
        let mut facts = Vec::new();
        let mut allocations: HashMap<ValueId, (Type, MemoryDomain)> = HashMap::new();

        // 1. Collect all allocations in the function
        for block in &func.blocks {
            for inst in &block.instructions {
                match &inst.op {
                    Op::Alloc { domain, ty } => {
                        if *domain != MemoryDomain::Stack {
                            allocations.insert(inst.result, (ty.clone(), *domain));
                        }
                    }
                    Op::Call { func_name, .. } if func_name == "syntra_arena_alloc" => {
                        allocations.insert(inst.result, (inst.ty.clone(), MemoryDomain::Arena));
                    }
                    Op::Call { func_name, .. } if func_name.ends_with("_new") => {
                        allocations.insert(
                            inst.result,
                            (inst.ty.clone(), MemoryDomain::ReferenceCounted),
                        );
                    }
                    _ => {}
                }
            }
        }

        if allocations.is_empty() {
            return facts;
        }

        // 2. Track alias set for each allocation
        // An alias is any ValueId derived from copying, loading, or passing the allocated value
        for (alloc_val, (ty, domain)) in allocations {
            let mut aliases = HashSet::new();
            aliases.insert(alloc_val);

            // Flow-sensitive alias propagation within function blocks
            let mut changed = true;
            while changed {
                changed = false;
                for block in &func.blocks {
                    for inst in &block.instructions {
                        match &inst.op {
                            Op::Store { ptr, val } => {
                                if aliases.contains(val) && aliases.insert(*ptr) {
                                    changed = true;
                                }
                            }
                            Op::Load { ptr } => {
                                if aliases.contains(ptr) && aliases.insert(inst.result) {
                                    changed = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // 3. Check for escape conditions
            let mut escape_reason = None;

            for block in &func.blocks {
                // Check block instructions
                for inst in &block.instructions {
                    match &inst.op {
                        Op::SetField {
                            object,
                            val,
                            field_name,
                            ..
                        } => {
                            if aliases.contains(val) && !aliases.contains(object) {
                                escape_reason =
                                    Some(EscapeReason::StoredToHeap(field_name.clone()));
                                break;
                            }
                        }
                        Op::SetIndex { array, val, .. } => {
                            if aliases.contains(val) && !aliases.contains(array) {
                                escape_reason = Some(EscapeReason::StoredToArray);
                                break;
                            }
                        }
                        Op::Call { func_name, args } => {
                            let mut passed_to_call = false;
                            for arg in args {
                                if aliases.contains(arg) {
                                    passed_to_call = true;
                                    break;
                                }
                            }

                            if passed_to_call {
                                if func_name == "syntra_task_spawn"
                                    || func_name == "syntra_task_spawn_n"
                                {
                                    escape_reason = Some(EscapeReason::ThreadEscape);
                                    break;
                                }
                                // If calling print or safe local methods that don't leak, we don't treat it as escaping
                                let is_safe_observer = func_name.starts_with("syntra_print")
                                    || func_name == "syntra_int_to_str"
                                    || func_name == "syntra_bool_to_str";

                                if !is_safe_observer {
                                    escape_reason =
                                        Some(EscapeReason::PassedToCall(func_name.clone()));
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if escape_reason.is_some() {
                    break;
                }

                // Check terminator (e.g. Return)
                if let Terminator::Return(Some(ret_val), _) = &block.terminator {
                    if aliases.contains(ret_val) {
                        escape_reason = Some(EscapeReason::Returned);
                        break;
                    }
                }
            }

            let (escape_state, promotable) = match escape_reason {
                Some(reason) => (EscapeState::Escaped(reason), false),
                None => (EscapeState::NoEscape, true),
            };

            let violates_arena = domain == MemoryDomain::Arena
                && !func.name.ends_with("_new")
                && matches!(
                    &escape_state,
                    EscapeState::Escaped(EscapeReason::Returned)
                        | EscapeState::Escaped(EscapeReason::ThreadEscape)
                        | EscapeState::Escaped(EscapeReason::StoredToHeap(_))
                        | EscapeState::Escaped(EscapeReason::StoredToArray)
                );

            facts.push(AllocationFact {
                value_id: alloc_val,
                function_name: func.name.clone(),
                ty,
                domain,
                status: ProofStatus::Proven,
                escape_state,
                promotable_to_stack: promotable,
                violates_arena_lifetime: violates_arena,
            });
        }

        facts
    }
}
