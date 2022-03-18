use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UpdateFavoriteRequest {
    pub id: i64,
}
