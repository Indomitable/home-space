use leptos::{prelude::*, task::spawn_local};

use api_client::{ApiError, format::human_size};
use protocol::{FileVersion, Node, NodeId, NodeKind, TicketRequest};

use crate::{icon::Icon, locale::short_date, session};

const FOLDER_LABEL: &str = "Folder";

async fn fetch(id: NodeId) -> Result<(Node, Vec<FileVersion>), ApiError> {
    let client = session::client();
    let node = client.get(id).await?;
    let versions = match node.kind {
        NodeKind::File => client.versions(id).await?,
        NodeKind::Folder => Vec::new(),
    };
    Ok((node, versions))
}

/// A panel beside the listing with the details of one node and the previous versions of a file.
#[component]
pub fn DetailsPanel(
    id: NodeId,
    details: RwSignal<Option<NodeId>>,
    refresh: RwSignal<u32>,
    auth: session::Auth,
) -> impl IntoView {
    let state = RwSignal::new(None::<(Node, Vec<FileVersion>)>);
    let error = RwSignal::new(None::<String>);
    let confirming = RwSignal::new(None::<i32>);

    Effect::new(move |_| {
        refresh.track();
        spawn_local(async move {
            match fetch(id).await {
                Ok(loaded) => {
                    state.set(Some(loaded));
                    error.set(None);
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    });

    let download = move |version: i32| {
        spawn_local(async move {
            let client = session::client();
            let request = TicketRequest::DownloadVersion {
                node_id: id,
                version,
            };
            match client.ticket(&request).await {
                Ok(ticket) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href(&client.version_download_url(
                            id,
                            version,
                            &ticket.ticket,
                        ));
                    }
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    };
    let delete = move |version: i32| {
        confirming.set(None);
        spawn_local(async move {
            if let Err(err) = session::client().delete_version(id, version).await {
                error.set(Some(auth.fail(&err)));
            }
            refresh.update(|count| *count += 1);
        });
    };

    view! {
        <aside class="details" aria-label="Details">
            <header class="details__header">
                <h2 class="details__title">
                    {move || state.read().as_ref().map(|(node, _)| node.name.clone())}
                </h2>
                <button
                    class="button button--ghost button--small"
                    aria-label="Close details"
                    on:click=move |_| details.set(None)
                >
                    <Icon name="close" />
                </button>
            </header>
            {move || {
                error
                    .get()
                    .map(|message| {
                        view! { <p class="alert alert--error" role="alert">{message}</p> }
                    })
            }}
            {move || {
                state
                    .get()
                    .map(|(node, versions)| {
                        let is_file = node.kind == NodeKind::File;
                        let kind = if is_file { node.mime.clone() } else { FOLDER_LABEL.to_owned() };
                        view! {
                            <dl class="details__list">
                                <dt>"Type"</dt>
                                <dd>{kind}</dd>
                                {is_file
                                    .then(|| {
                                        view! {
                                            <dt>"Size"</dt>
                                            <dd>{human_size(node.size)}</dd>
                                            <dt>"Version"</dt>
                                            <dd>{node.version}</dd>
                                        }
                                    })}
                                <dt>"Created"</dt>
                                <dd>{short_date(node.created_at)}</dd>
                                <dt>"Modified"</dt>
                                <dd>{short_date(node.updated_at)}</dd>
                            </dl>
                            {is_file
                                .then(|| {
                                    view! {
                                        <section class="details__versions">
                                            <h3 class="details__subtitle">"Previous versions"</h3>
                                            {if versions.is_empty() {
                                                view! {
                                                    <p class="details__empty">"No previous versions."</p>
                                                }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <ul class="details__version-list">
                                                        {versions
                                                            .into_iter()
                                                            .map(|version| {
                                                                view! {
                                                                    <VersionRow
                                                                        version
                                                                        confirming
                                                                        on_download=Callback::new(download)
                                                                        on_delete=Callback::new(delete)
                                                                    />
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </ul>
                                                }
                                                    .into_any()
                                            }}
                                        </section>
                                    }
                                })}
                        }
                    })
            }}
        </aside>
    }
}

#[component]
fn VersionRow(
    version: FileVersion,
    confirming: RwSignal<Option<i32>>,
    on_download: Callback<i32>,
    on_delete: Callback<i32>,
) -> impl IntoView {
    let number = version.version;
    let asking = move || confirming.get() == Some(number);
    view! {
        <li class="details__version">
            <div class="details__version-info">
                <span class="details__version-number">{format!("Version {number}")}</span>
                <span class="details__version-meta">
                    {format!("{} · {}", human_size(version.size), short_date(version.modified_at))}
                </span>
            </div>
            <div class="details__version-actions">
                {move || {
                    if asking() {
                        view! {
                            <button
                                class="button button--danger button--small"
                                on:click=move |_| on_delete.run(number)
                            >
                                "Delete"
                            </button>
                            <button
                                class="button button--ghost button--small"
                                on:click=move |_| confirming.set(None)
                            >
                                "Cancel"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button
                                class="button button--ghost button--small"
                                aria-label=format!("Download version {number}")
                                on:click=move |_| on_download.run(number)
                            >
                                <Icon name="download" />
                            </button>
                            <button
                                class="button button--ghost button--small"
                                aria-label=format!("Delete version {number}")
                                on:click=move |_| confirming.set(Some(number))
                            >
                                <Icon name="delete" />
                            </button>
                        }
                            .into_any()
                    }
                }}
            </div>
        </li>
    }
}
