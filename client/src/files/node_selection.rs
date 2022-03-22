use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone)]
pub struct NodeSelection {
    selected: HashSet<i64>    
}

impl NodeSelection {
    pub fn new() -> Self {
        Self {
            selected: HashSet::new()
        }
    }

    pub fn toggle_selction(&mut self, node_id: i64) {
        if self.selected.contains(&node_id) {
            self.selected.remove(&node_id);
        } else {
            self.selected.insert(node_id);
        }
    }

    pub fn has(&self, node_id: &i64) -> bool {
        self.selected.contains(&node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::NodeSelection;

    #[test]
    fn should_add_item_if_missing() {
        let mut selection = NodeSelection::new();
        assert_eq!(selection.has(&1), false);

        selection.toggle_selction(1);
        assert_eq!(selection.has(&1), true);
    }

    #[test]
    fn should_remove_item_when_inside() {
        let mut selection = NodeSelection::new();

        selection.toggle_selction(1);
        assert_eq!(selection.has(&1), true);

        selection.toggle_selction(1);
        assert_eq!(selection.has(&1), false);
    }
}
