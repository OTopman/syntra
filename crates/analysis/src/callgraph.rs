use std::collections::{HashMap, HashSet};
use syntra_air::ir::{AirModule, Op};

#[derive(Debug, Clone)]
pub struct CallNode {
    pub name: String,
    pub callees: HashSet<String>,
    pub callers: HashSet<String>,
    pub is_leaf: bool,
    pub is_recursive: bool,
}

#[derive(Debug, Clone)]
pub struct CallGraph {
    pub nodes: HashMap<String, CallNode>,
    pub entry_points: Vec<String>,
}

impl CallGraph {
    pub fn build(module: &AirModule) -> Self {
        let mut nodes: HashMap<String, CallNode> = HashMap::new();

        // 1. Initialize nodes for all functions
        for func in &module.functions {
            nodes.insert(
                func.name.clone(),
                CallNode {
                    name: func.name.clone(),
                    callees: HashSet::new(),
                    callers: HashSet::new(),
                    is_leaf: true,
                    is_recursive: false,
                },
            );
        }

        // 2. Discover edges from instructions
        for func in &module.functions {
            let mut callees = HashSet::new();

            for block in &func.blocks {
                for inst in &block.instructions {
                    match &inst.op {
                        Op::Call { func_name, .. } => {
                            callees.insert(func_name.clone());
                        }
                        Op::FuncAddr { func_name } => {
                            callees.insert(func_name.clone());
                        }
                        _ => {}
                    }
                }
            }

            for callee in &callees {
                if callee == &func.name {
                    if let Some(node) = nodes.get_mut(&func.name) {
                        node.is_recursive = true;
                    }
                }
                if let Some(callee_node) = nodes.get_mut(callee) {
                    callee_node.callers.insert(func.name.clone());
                }
            }

            if let Some(node) = nodes.get_mut(&func.name) {
                node.is_leaf = callees.is_empty();
                node.callees = callees;
            }
        }

        let mut entry_points = Vec::new();
        for (name, node) in &nodes {
            if name == "main" || node.callers.is_empty() {
                entry_points.push(name.clone());
            }
        }
        entry_points.sort();

        Self {
            nodes,
            entry_points,
        }
    }

    pub fn is_reachable(&self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }
        let mut visited = HashSet::new();
        let mut queue = vec![from.to_string()];

        while let Some(current) = queue.pop() {
            if current == to {
                return true;
            }
            if visited.insert(current.clone()) {
                if let Some(node) = self.nodes.get(&current) {
                    for callee in &node.callees {
                        if !visited.contains(callee) {
                            queue.push(callee.clone());
                        }
                    }
                }
            }
        }

        false
    }
}
