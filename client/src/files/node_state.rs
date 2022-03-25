use std::collections::HashMap;

use home_space_contracts::files::DisplayFileNode;

#[derive(Debug, PartialEq, Clone)]
pub struct NodeState {
    pub is_selected: bool
}

impl Default for NodeState {
    fn default() -> Self {
        Self { is_selected: false }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct NodesState {
    pub states: HashMap<i64, NodeState>
}

impl NodesState {
    pub(crate) fn new() -> NodesState {
        Self {
            states: HashMap::new()
        }
    }

    pub(crate) fn fill_default(&mut self, nodes: &Vec<DisplayFileNode>) {
        self.states.clear();
        for node in nodes {
            self.states.insert(node.id, NodeState::default());
        }
    }
}