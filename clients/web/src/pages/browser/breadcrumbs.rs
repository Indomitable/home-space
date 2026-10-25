use std::ops::Range;

use leptos::prelude::*;
use leptos_router::components::A;

use protocol::Node;

use super::menu::{Menu, MenuLink};
use crate::route;

const MAX_VISIBLE: usize = 3;
const OVERFLOW_ICON: &str = "";
const OVERFLOW_TEXT: &str = "…";

/// The crumbs hidden behind the overflow menu once the path is deeper than [`MAX_VISIBLE`].
fn collapsed(len: usize) -> Option<Range<usize>> {
    (len > MAX_VISIBLE).then(|| 1..len - 1)
}

#[component]
pub fn Breadcrumbs(crumbs: RwSignal<Vec<Node>>) -> impl IntoView {
    view! {
        <nav class="breadcrumbs" aria-label="Folder path">
            <A href="/" exact=true attr:class="breadcrumbs__link">
                "Home"
            </A>
            {move || {
                let all = crumbs.get();
                let hidden = collapsed(all.len()).unwrap_or_default();
                let overflow = all.get(hidden.clone()).map(<[Node]>::to_vec);
                all.into_iter()
                    .enumerate()
                    .filter_map(|(index, folder)| {
                        if index == hidden.start && !hidden.is_empty() {
                            overflow.clone().map(|folders| view! { <Overflow crumbs=folders /> }.into_any())
                        } else if hidden.contains(&index) {
                            None
                        } else {
                            Some(view! { <Crumb folder /> }.into_any())
                        }
                    })
                    .collect_view()
            }}
        </nav>
    }
}

#[component]
fn Crumb(folder: Node) -> impl IntoView {
    view! {
        <span class="breadcrumbs__separator" aria-hidden="true">
            "/"
        </span>
        <A href=route::folder(folder.id) exact=true attr:class="breadcrumbs__link">
            {folder.name}
        </A>
    }
}

#[component]
fn Overflow(crumbs: Vec<Node>) -> impl IntoView {
    let crumbs = StoredValue::new(crumbs);
    view! {
        <span class="breadcrumbs__separator" aria-hidden="true">
            "/"
        </span>
        <Menu
            id="crumbs-overflow".to_owned()
            label="Show hidden folders"
            text=Signal::stored(OVERFLOW_TEXT.to_owned())
            icon=OVERFLOW_ICON
        >
            {move || {
                crumbs
                    .get_value()
                    .into_iter()
                    .map(|folder| {
                        view! { <MenuLink href=route::folder(folder.id)>{folder.name}</MenuLink> }
                    })
                    .collect_view()
            }}
        </Menu>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_paths_are_not_collapsed() {
        assert_eq!((collapsed(0), collapsed(3)), (None, None));
    }

    #[test]
    fn deep_paths_hide_the_middle_folders() {
        assert_eq!((collapsed(4), collapsed(6)), (Some(1..3), Some(1..5)));
    }
}
