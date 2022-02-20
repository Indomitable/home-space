#![allow(dead_code)]

use std::env;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use deadpool_postgres::tokio_postgres::NoTls;
use deadpool_postgres::tokio_postgres::types::ToSql;
use actix_web::web;

pub type DbResult<T> = std::result::Result<T, deadpool_postgres::PoolError>;

pub fn new_pool() -> Pool {
    let server = env::var("DB_SERVER_NAME").unwrap();
    let database = env::var("DB_SERVER_DATABASE").unwrap();
    let user_name = env::var("DB_SERVER_USER_NAME").unwrap();
    let password = env::var("DB_SERVER_PASSWORD").unwrap();
    let connection = format!("postgresql://{}:{}@{}/{}?connect_timeout=10&application_name=home-space", user_name, password, server, database);
    let config: deadpool_postgres::tokio_postgres::Config = connection.as_str().parse().unwrap();
    let manager_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast
    };
    let manager = Manager::from_config(config, NoTls, manager_config);
    let pool = Pool::builder(manager).max_size(12).build().unwrap();
    return pool;
}

pub async fn query(pool: &web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<Vec<deadpool_postgres::tokio_postgres::Row>> {
    let connection = pool.get().await?;
    let statement = connection.prepare_cached(query).await?;
    let rows = connection.query(&statement, params).await?;
    return Ok(rows);
}

pub async fn query_one(pool: &web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<deadpool_postgres::tokio_postgres::Row> {
    let connection = pool.get().await?;
    let statement = connection.prepare_cached(query).await?;
    let row = connection.query_one(&statement, params).await?;
    return Ok(row);
}

pub async fn execute(pool: &web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> DbResult<u64> {
    let connection = pool.get().await?;
    let statement = connection.prepare_cached(query).await?;
    let affected = connection.execute(&statement, params).await?;
    return Ok(affected);
}
