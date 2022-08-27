use std::sync::Arc;
use deadpool_postgres::{Client, Manager, ManagerConfig, Pool, RecyclingMethod, Transaction};
use deadpool_postgres::tokio_postgres::{NoTls, RowStream};
use deadpool_postgres::tokio_postgres::types::ToSql;

use crate::config::get_db_connection_url;

#[derive(Debug)]
pub enum DbError {
    OpenConnection(String),
    PrepareSql(String),
    Execute(String),
    Fetch(String),
    StartTransaction(String),
    CommitTransaction(String),
    RollbackTransaction(String),
}

pub type DbResult<T> = Result<T, DbError>;

pub fn new_pool() -> Pool {
    let connection_url = get_db_connection_url();
    let config: deadpool_postgres::tokio_postgres::Config = connection_url.as_str().parse().unwrap();
    let manager_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast
    };
    let manager = Manager::from_config(config, NoTls, manager_config);
    let pool = Pool::builder(manager).max_size(12).build().unwrap();
    return pool;
}

pub(crate) struct DatabaseAccess {
    pool: Arc<Pool>
}

impl DatabaseAccess {
    pub(crate) fn new(pool: &Arc<Pool>) -> Self {
        Self {
            pool: Arc::clone(pool)
        }
    }

    pub(crate) async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<Vec<deadpool_postgres::tokio_postgres::Row>> {
        let connection = self.get_connection().await?;
        let statement = self.prepare_statement(&connection, query).await?;
        return match connection.query(&statement, params).await {
            Ok(rows) => {
                Ok(rows)
            },
            Err(error) => {
                log::error!("Can not query multiple rows! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn query_raw(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<RowStream> {
        let connection = self.get_connection().await?;
        let statement = self.prepare_statement(&connection, query).await?;
        return match connection.query_raw(&statement, slice_iter(params)).await {
            Ok(rows_stream) => {
                Ok(rows_stream)
            },
            Err(error) => {
                log::error!("Can not query raw multiple rows! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<deadpool_postgres::tokio_postgres::Row> {
        let connection = self.get_connection().await?;
        let statement = self.prepare_statement(&connection, query).await?;
        return match connection.query_one(&statement, params).await {
            Ok(row) => {
                Ok(row)
            },
            Err(error) => {
                log::error!("Can not query single row! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn query_opt(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<Option<deadpool_postgres::tokio_postgres::Row>> {
        let connection = self.get_connection().await?;
        let statement = self.prepare_statement(&connection, query).await?;
        return match connection.query_opt(&statement, params).await {
            Ok(row) => {
                Ok(row)
            },
            Err(error) => {
                log::error!("Can not query optional single row! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<u64> {
        let connection = self.get_connection().await?;
        let statement = self.prepare_statement(&connection, query).await?;
        return match connection.execute(&statement, params).await {
            Ok(affected) => {
                Ok(affected)
            },
            Err(error) => {
                log::error!("Can not execute statement! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn create_db_connection(&mut self) -> DbResult<DbConnection> {
        let connection = self.get_connection().await?;
        Ok(DbConnection { client: connection })
    }

    async fn get_connection(&self) -> Result<deadpool_postgres::Object, DbError> {
        return match self.pool.get().await {
            Ok(connection) => {
                Ok(connection)
            },
            Err(error) => {
                log::error!("Can not create db connection!. [Error={}]", error);
                Err(DbError::OpenConnection(error.to_string()))
            }
        }
    }

    async fn prepare_statement(&self, connection: &deadpool_postgres::Object, query: &str) -> Result<deadpool_postgres::tokio_postgres::Statement, DbError> {
        return match connection.prepare_cached(query).await {
            Ok(statement) => {
                Ok(statement)
            },
            Err(error) => {
                log::error!("Can not prepare statement!. [Error={}]", error);
                Err(DbError::PrepareSql(error.to_string()))
            }
        }
    }
}

pub(crate) struct DbConnection {
    client: Client
}

impl DbConnection {
    pub(crate) async fn create_transaction(&mut self) -> DbResult<TransactionalDataAccess> {
        let trans = self.client.transaction().await.map_err(|e| DbError::StartTransaction(e.to_string()))?;
        Ok(TransactionalDataAccess { transaction: trans })
    }
}

fn slice_iter<'a>(
    s: &'a [&'a (dyn ToSql + Sync)],
) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
    s.iter().map(|s| *s as _)
}

pub(crate) struct TransactionalDataAccess<'a> {
    transaction: Transaction<'a>,
}

// #[async_trait]
impl<'a> TransactionalDataAccess<'a> {
    pub(crate) async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<deadpool_postgres::tokio_postgres::Row> {
        let statement = self.transaction.prepare(query).await.map_err(|er| DbError::PrepareSql(er.to_string()))?;
        return match self.transaction.query_one(&statement, params).await {
            Ok(row) => {
                Ok(row)
            },
            Err(error) => {
                log::error!("Can not query single row! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn query_opt(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<Option<deadpool_postgres::tokio_postgres::Row>> {
        let statement = self.transaction.prepare(query).await.map_err(|er| DbError::PrepareSql(er.to_string()))?;
        return match self.transaction.query_opt(&statement, params).await {
            Ok(row) => {
                Ok(row)
            },
            Err(error) => {
                log::error!("Can not query optional single row! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<u64> {
        let statement = self.transaction.prepare(query).await.map_err(|er| DbError::PrepareSql(er.to_string()))?;
        return match self.transaction.execute(&statement, params).await {
            Ok(affected) => {
                Ok(affected)
            },
            Err(error) => {
                log::error!("Can not execute statement! [Error={}]", error);
                Err(DbError::Execute(error.to_string()))
            }
        }
    }

    pub(crate) async fn commit(self) -> DbResult<()> {
        self.transaction.commit().await.map_err(|e| DbError::CommitTransaction(e.to_string()))
    }

    pub(crate) async fn rollback(self) -> DbResult<()> {
        self.transaction.rollback().await.map_err(|e| DbError::RollbackTransaction(e.to_string()))
    }
}
