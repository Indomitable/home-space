use protocol::NodeId;

/// How many of the listed nodes are selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    None,
    Some,
    All,
}

/// How much of `listed` is selected; an empty listing covers nothing.
pub fn coverage(
    listed: impl IntoIterator<Item = NodeId>,
    is_selected: impl Fn(NodeId) -> bool,
) -> Coverage {
    let (mut total, mut selected) = (0, 0);
    for id in listed {
        total += 1;
        if is_selected(id) {
            selected += 1;
        }
    }
    if selected == 0 {
        Coverage::None
    } else if selected == total {
        Coverage::All
    } else {
        Coverage::Some
    }
}

/// Whether the toolbar swaps its usual commands for the bulk actions: only once several
/// nodes are selected, since a single node has its own row actions.
pub fn shows_bulk_actions(selected: usize) -> bool {
    selected > 1
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use uuid::Uuid;

    use super::*;

    fn ids(count: u128) -> Vec<NodeId> {
        (1..=count)
            .map(|n| NodeId::from(Uuid::from_u128(n)))
            .collect()
    }

    fn of(listed: &[NodeId], selected: &HashSet<NodeId>) -> Coverage {
        coverage(listed.iter().copied(), |id| selected.contains(&id))
    }

    #[test]
    fn nothing_selected_is_none() {
        assert_eq!(of(&ids(3), &HashSet::new()), Coverage::None);
    }

    #[test]
    fn an_empty_listing_is_none() {
        assert_eq!(of(&[], &HashSet::new()), Coverage::None);
    }

    #[test]
    fn part_selected_is_some() {
        let ids = ids(3);
        assert_eq!(of(&ids, &HashSet::from([ids[0]])), Coverage::Some);
    }

    #[test]
    fn every_row_selected_is_all() {
        let ids = ids(3);
        let selected = ids.iter().copied().collect();
        assert_eq!(of(&ids, &selected), Coverage::All);
    }

    #[test]
    fn selection_outside_the_listing_is_ignored() {
        let selected = HashSet::from([NodeId::from(Uuid::from_u128(99))]);
        assert_eq!(of(&ids(3), &selected), Coverage::None);
    }

    #[test]
    fn bulk_actions_show_only_for_several_nodes() {
        assert!(!shows_bulk_actions(0));
        assert!(!shows_bulk_actions(1));
        assert!(shows_bulk_actions(2));
    }
}
