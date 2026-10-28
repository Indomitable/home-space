use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use api_client::{
    ApiError,
    listing::{Sort, affects_path, apply_event, sort_nodes},
};
use protocol::{Event, Node, NodeId};

use crate::{
    events, load, route,
    session::{self, Auth},
};

type Loaded = Result<(Vec<Node>, Vec<Node>), ApiError>;

async fn fetch(folder: Option<NodeId>) -> Loaded {
    let client = session::client();
    let listing = client.list(folder).await?;
    let path = match folder {
        Some(id) => {
            let mut path = client.ancestors(id).await?;
            path.push(client.get(id).await?);
            path
        }
        None => Vec::new(),
    };
    Ok((listing, path))
}

/// The open folder: its contents and path, loaded from the route and kept current by
/// live events.
#[derive(Clone, Copy)]
pub(super) struct Listing {
    pub folder: Memo<Option<NodeId>>,
    pub nodes: RwSignal<Vec<Node>>,
    pub crumbs: RwSignal<Vec<Node>>,
    pub sort: RwSignal<Sort>,
    pub error: RwSignal<Option<String>>,
    auth: Auth,
    resource: LocalResource<Loaded>,
}

impl Listing {
    pub fn new(auth: Auth) -> Self {
        let params = use_params_map();
        let folder =
            Memo::new(move |_| params.read().get("id").and_then(|id| route::parse_id(&id)));
        let nodes = RwSignal::new(Vec::<Node>::new());
        let crumbs = RwSignal::new(Vec::<Node>::new());
        let sort = RwSignal::new(Sort::default());
        let error = RwSignal::new(None::<String>);

        let resource = load::load(
            move || fetch(folder.get()),
            move |loaded: Loaded| match loaded {
                Ok((mut listing, path)) => {
                    sort_nodes(&mut listing, sort.get_untracked());
                    nodes.set(listing);
                    crumbs.set(path);
                    error.set(None);
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            },
        );

        Effect::new(move |_| {
            let sort = sort.get();
            nodes.update(|listing| sort_nodes(listing, sort));
        });

        events::use_events(move |event| {
            if matches!(event, Event::Resync) || affects_path(&crumbs.get_untracked(), &event) {
                resource.refetch();
            } else {
                nodes.update(|listing| {
                    apply_event(
                        listing,
                        folder.get_untracked(),
                        &event,
                        sort.get_untracked(),
                    )
                });
            }
        });

        Self {
            folder,
            nodes,
            crumbs,
            sort,
            error,
            auth,
            resource,
        }
    }

    /// Loads the folder again.
    pub fn refetch(self) {
        self.resource.refetch();
    }

    /// Shows the error of a finished change, if any, and loads the folder again.
    pub fn settle(self, result: Result<(), ApiError>) {
        if let Err(err) = result {
            self.error.set(Some(self.auth.fail(&err)));
        }
        self.refetch();
    }
}
