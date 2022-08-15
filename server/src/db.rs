#![allow(dead_code)]

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use deadpool_postgres::tokio_postgres::NoTls;
use deadpool_postgres::tokio_postgres::types::ToSql;
use actix_web::web;

use crate::config::get_db_connection_url;

#[derive(Debug)]
pub enum DbError {
    OpenConnection(String),
    PrepareSql(String),
    Execute(String)
}

pub type DbResult<T> = std::result::Result<T, DbError>;

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

pub async fn query(pool: &web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<Vec<deadpool_postgres::tokio_postgres::Row>> {
    let connection = get_connection(pool).await?;
    let statement = prepare_statement(&connection, query).await?;
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

pub async fn query_one(pool: &web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<deadpool_postgres::tokio_postgres::Row> {
    let connection = get_connection(pool).await?;
    let statement = prepare_statement(&connection, query).await?;
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

pub async fn query_opt(pool: &web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<Option<deadpool_postgres::tokio_postgres::Row>> {
    let connection = get_connection(pool).await?;
    let statement = prepare_statement(&connection, query).await?;
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

pub async fn execute(pool: &web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<u64> {
    let connection = get_connection(pool).await?;
    let statement = prepare_statement(&connection, query).await?;
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

async fn get_connection(pool: &web::Data<Pool>) -> Result<deadpool_postgres::Object, DbError> {
    return match pool.get().await {
        Ok(connection) => {
            Ok(connection)
        },
        Err(error) => {
            log::error!("Can not create db connection!. [Error={}]", error);
            Err(DbError::OpenConnection(error.to_string()))
        }
    }
}

async fn prepare_statement(connection: &deadpool_postgres::Object, query: &str) -> Result<deadpool_postgres::tokio_postgres::Statement, DbError> {
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
