use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct UpdateFavoriteRequest {
    pub id: i64,
}
