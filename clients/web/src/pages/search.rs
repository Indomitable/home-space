use leptos::{prelude::*, task::spawn_local};
use leptos_router::{components::A, hooks::use_query_map};

use api_client::format::human_size;
use protocol::{NodeKind, SearchHit};

use crate::{
    icon::{Icon, node_icon},
    locale::short_date,
    route,
    session::{self, Auth},
};

const HOME_LABEL: &str = "Home";
const LOCATION_SEPARATOR: &str = " / ";
const HOME_PATH: &str = "/";

#[derive(Clone, PartialEq)]
enum Status {
    Idle,
    Loading,
    Done,
    Failed(String),
}

fn target(hit: &SearchHit) -> String {
    match (hit.node.kind, hit.node.parent_id) {
        (NodeKind::Folder, _) => route::folder(hit.node.id),
        (NodeKind::File, Some(parent)) => route::folder(parent),
        (NodeKind::File, None) => HOME_PATH.to_owned(),
    }
}

fn location_label(location: &[String]) -> String {
    std::iter::once(HOME_LABEL)
        .chain(location.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(LOCATION_SEPARATOR)
}

#[component]
pub fn SearchPage(auth: Auth) -> impl IntoView {
    let params = use_query_map();
    let term = Memo::new(move |_| {
        params
            .read()
            .get(route::SEARCH_PARAM)
            .unwrap_or_default()
            .trim()
            .to_owned()
    });
    let hits = RwSignal::new(Vec::<SearchHit>::new());
    let status = RwSignal::new(Status::Idle);

    Effect::new(move |_| {
        let wanted = term.get();
        if wanted.is_empty() {
            hits.set(Vec::new());
            status.set(Status::Idle);
        } else {
            status.set(Status::Loading);
            spawn_local(async move {
                let outcome = session::client().search(&wanted).await;
                if term.get_untracked() == wanted {
                    match outcome {
                        Ok(found) => {
                            hits.set(found);
                            status.set(Status::Done);
                        }
                        Err(err) => status.set(Status::Failed(auth.fail(&err))),
                    }
                }
            });
        }
    });

    view! {
        <main class="explorer">
            <div class="explorer__header">
                <h2 class="search-title">
                    {move || {
                        let term = term.get();
                        if term.is_empty() {
                            "Search".to_owned()
                        } else {
                            format!("Results for \u{201c}{term}\u{201d}")
                        }
                    }}
                </h2>
            </div>
            {move || match status.get() {
                Status::Failed(message) => {
                    Some(view! { <p class="alert alert--error" role="alert">{message}</p> })
                }
                _ => None,
            }}
            <div class="explorer__scroll">
                <Show when=move || !hits.read().is_empty()>
                    <table class="node-table">
                        <thead class="node-table__head">
                            <tr>
                                <th class="node-table__header-cell">"Name"</th>
                                <th class="node-table__header-cell">"Location"</th>
                                <th class="node-table__header-cell node-table__header-cell--size">
                                    "Size"
                                </th>
                                <th class="node-table__header-cell node-table__header-cell--modified">
                                    "Modified"
                                </th>
                            </tr>
                        </thead>
                        <tbody class="node-table__body">
                            <For
                                each=move || hits.get()
                                key=|hit| (hit.node.id, hit.node.updated_at.unix_timestamp_nanos())
                                children=|hit| view! { <HitRow hit /> }
                            />
                        </tbody>
                    </table>
                </Show>
                <Show when=move || status.get() == Status::Loading && hits.read().is_empty()>
                    <p class="explorer__empty">"Searching\u{2026}"</p>
                </Show>
                <Show when=move || status.get() == Status::Done && hits.read().is_empty()>
                    <p class="explorer__empty">"Nothing matches your search."</p>
                </Show>
                <Show when=move || status.get() == Status::Idle>
                    <p class="explorer__empty">"Type a file or folder name to search."</p>
                </Show>
            </div>
        </main>
    }
}

#[component]
fn HitRow(hit: SearchHit) -> impl IntoView {
    let href = target(&hit);
    let icon = node_icon(&hit.node);
    let is_folder = hit.node.kind == NodeKind::Folder;
    let size = if is_folder {
        String::new()
    } else {
        human_size(hit.node.size)
    };
    let modified = short_date(hit.node.updated_at);
    let location = location_label(&hit.location);
    let name = hit.node.name;

    view! {
        <tr class="node-table__row">
            <td class="node-table__cell">
                <A href=href attr:class="node-table__name node-table__name--link">
                    <Icon name=icon />
                    {name}
                </A>
            </td>
            <td class="node-table__cell search-results__location">{location}</td>
            <td class="node-table__cell node-table__cell--size">{size}</td>
            <td class="node-table__cell node-table__cell--modified">{modified}</td>
        </tr>
    }
}
