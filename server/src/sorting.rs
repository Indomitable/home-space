use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Sorting {
    column_name: String,
    direction: SortDirection
}

impl Sorting {
    pub fn build_order_by(&self) -> String {
        let sort_direction = match self.direction {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        };
        format!("{} {}", self.column_name, sort_direction)
    }
}
