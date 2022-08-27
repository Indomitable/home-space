pub(crate) mod search_query_creator;

pub struct SearchModel {
    pub title: Option<String>,
    pub parent_id: Option<i64>,
    pub node_type: Option<i16>,
    pub mime_type: Option<String>,
    pub from_date: Option<chrono::DateTime<chrono::Utc>>,
    pub to_date: Option<chrono::DateTime<chrono::Utc>>,
    pub from_size: Option<i64>,
    pub to_size: Option<i64>,    
}
