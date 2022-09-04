use deadpool_postgres::tokio_postgres::Row;

pub(crate) mod file_node;
pub(crate) mod file_version;
pub(crate) mod deleted_node;

pub(super) trait DbModel {
    fn read_node(row: &Row) -> Self;

    fn column_list(table_alias: &str) -> String;
}
