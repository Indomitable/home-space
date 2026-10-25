use auth::AuthService;
use db::Db;
use files::FileService;
use notifications::Hub;
use search::SearchIndex;

/// Shared services handed to every handler.
#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Db,
    pub auth: AuthService,
    pub files: FileService,
    pub hub: Hub,
    pub search: SearchIndex,
}

impl AppState {
    /// Indexes a node's name; a failure is logged because the node change itself succeeded and
    /// the next catch-up pass repairs the index.
    pub(crate) async fn index_name(&self, node: &db::Node) {
        if let Err(err) = self.search.index_name(node).await {
            tracing::warn!(node_id = %node.id, error = %err, "failed to index node name");
        }
    }
}
