//! Row selection shared by the tables that list nodes.

use std::collections::HashSet;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, MouseEvent};

use protocol::NodeId;

use crate::icon::Icon;

const ICON_CHECKED: &str = "check_box";
const ICON_UNCHECKED: &str = "check_box_outline_blank";
const ICON_MIXED: &str = "indeterminate_check_box";

/// Clicks inside these elements belong to them, not to the row.
const INTERACTIVE_SELECTOR: &str = "a, button, input, .node-table__cell--actions";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Selection {
    None,
    Some,
    All,
}

fn selection_of(ids: &[NodeId], selected: &HashSet<NodeId>) -> Selection {
    let count = ids.iter().filter(|id| selected.contains(id)).count();
    if count == 0 {
        Selection::None
    } else if count == ids.len() {
        Selection::All
    } else {
        Selection::Some
    }
}

fn toggle(selected: RwSignal<HashSet<NodeId>>, id: NodeId) {
    selected.update(|ids| {
        if !ids.remove(&id) {
            ids.insert(id);
        }
    });
}

/// Handles a click on the row of `id`: a plain click selects only that row, and a click
/// with Ctrl, Cmd or Shift toggles it. Clicks on links, buttons and inputs are left alone.
pub fn select_on_click(
    id: NodeId,
    selected: RwSignal<HashSet<NodeId>>,
) -> impl Fn(MouseEvent) + Copy + 'static {
    move |event: MouseEvent| {
        let interactive = event
            .target()
            .and_then(|target| target.dyn_into::<Element>().ok())
            .is_some_and(|element| matches!(element.closest(INTERACTIVE_SELECTOR), Ok(Some(_))));
        if !interactive {
            if event.ctrl_key() || event.meta_key() || event.shift_key() {
                toggle(selected, id);
            } else {
                selected.set(HashSet::from([id]));
            }
        }
    }
}

#[component]
fn Checkbox(
    #[prop(into)] icon: Signal<&'static str>,
    #[prop(into)] checked: Signal<String>,
    #[prop(into)] label: Signal<String>,
    on_toggle: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            class="node-table__checkbox"
            role="checkbox"
            aria-checked=move || checked.get()
            aria-label=move || label.get()
            on:click=move |_| on_toggle.run(())
        >
            {move || view! { <Icon name=icon.get() /> }}
        </button>
    }
}

/// The header checkbox: selects every row in `ids`, or clears the selection once all are selected.
#[component]
pub fn SelectAll(
    #[prop(into)] ids: Signal<Vec<NodeId>>,
    selected: RwSignal<HashSet<NodeId>>,
) -> impl IntoView {
    let state = Memo::new(move |_| selection_of(&ids.read(), &selected.read()));
    let icon = move || match state.get() {
        Selection::None => ICON_UNCHECKED,
        Selection::Some => ICON_MIXED,
        Selection::All => ICON_CHECKED,
    };
    let checked = move || {
        match state.get() {
            Selection::None => "false",
            Selection::Some => "mixed",
            Selection::All => "true",
        }
        .to_owned()
    };
    let on_toggle = Callback::new(move |()| {
        if state.get_untracked() == Selection::All {
            selected.update(HashSet::clear);
        } else {
            selected.set(ids.get_untracked().into_iter().collect());
        }
    });
    view! { <Checkbox icon checked label="Select all" on_toggle /> }
}

/// The checkbox of the row of `id`, labelled with the row's `name`.
#[component]
pub fn RowCheckbox(id: NodeId, name: String, selected: RwSignal<HashSet<NodeId>>) -> impl IntoView {
    let is_selected = move || selected.read().contains(&id);
    let icon = move || {
        if is_selected() {
            ICON_CHECKED
        } else {
            ICON_UNCHECKED
        }
    };
    let checked = move || is_selected().to_string();
    view! {
        <Checkbox
            icon=Signal::derive(icon)
            checked=Signal::derive(checked)
            label=format!("Select {name}")
            on_toggle=Callback::new(move |()| toggle(selected, id))
        />
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn ids(count: u128) -> Vec<NodeId> {
        (1..=count)
            .map(|n| NodeId::from(Uuid::from_u128(n)))
            .collect()
    }

    #[test]
    fn nothing_selected_is_none() {
        assert_eq!(selection_of(&ids(3), &HashSet::new()), Selection::None);
    }

    #[test]
    fn an_empty_table_is_none() {
        assert_eq!(selection_of(&[], &HashSet::new()), Selection::None);
    }

    #[test]
    fn part_selected_is_some() {
        let ids = ids(3);
        let selected = HashSet::from([ids[0]]);
        assert_eq!(selection_of(&ids, &selected), Selection::Some);
    }

    #[test]
    fn every_row_selected_is_all() {
        let ids = ids(3);
        let selected = ids.iter().copied().collect();
        assert_eq!(selection_of(&ids, &selected), Selection::All);
    }

    #[test]
    fn selection_outside_the_table_is_ignored() {
        let ids = ids(3);
        let selected = HashSet::from([NodeId::from(Uuid::from_u128(99))]);
        assert_eq!(selection_of(&ids, &selected), Selection::None);
    }
}
