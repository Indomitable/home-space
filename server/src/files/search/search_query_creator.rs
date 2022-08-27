use postgres_types::ToSql;

use crate::files::db::{file_node::FileNodeDto, DbModel};

use super::SearchModel;

pub(crate) trait SearchQueryCreator {
    fn get_sql_query<'model>(&self, user_id: i64, model: &'model SearchModel) -> (String, Vec<Box<dyn ToSql + 'model>>);
}

pub(crate) struct SearchQueryCreatorImpl;


impl SearchQueryCreator for SearchQueryCreatorImpl {
    fn get_sql_query<'model>(&self, user_id: i64, model: &'model SearchModel) -> (String, Vec<Box<dyn ToSql + 'model>>) {
        let (w, p) = self.build_where_statement(user_id, model, "fn");
        let sql = format!("select {} from file_nodes fn where {}",
            FileNodeDto::column_list(), w);
        (sql, p)
    }
}

impl SearchQueryCreatorImpl {
    fn build_where_statement<'model>(&self, user_id: i64, model: &'model SearchModel, table_alias: &str) -> (String, Vec<Box<dyn ToSql + 'model>>) {
        let mut statements = vec![format!("{}.user_id = $1", table_alias)];
        let mut params: Vec<Box<dyn ToSql>> = vec![Box::new(user_id)];
        if let Some(title) = &model.title {
            params.push(Box::new(title));
            statements.push(format!("{}.title = ${}", table_alias, params.len()));
        }
        if let Some(parent) = &model.parent_id {
            params.push(Box::new(parent));
            statements.push(format!("{}.title = ${}", table_alias, params.len()));
        }
        (statements.join(" and "), params)
    }
}
