use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use deadpool_postgres::tokio_postgres::NoTls;
use deadpool_postgres::tokio_postgres::types::ToSql;
use actix_web::web;

pub fn new_pool() -> Pool {
    let connection = format!("postgresql://{}:{}@{}/{}?connect_timeout=10&application_name=home-space", "files", "files", "localhost", "files_db");
    let config: deadpool_postgres::tokio_postgres::Config = connection.as_str().parse().unwrap();
    let manager_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast
    };
    let manager = Manager::from_config(config, NoTls, manager_config);
    let pool = Pool::builder(manager).max_size(12).build().unwrap();
    return pool;
}

pub async fn query(pool: web::Data<Pool>, query: &str, params: &[&(dyn ToSql + Sync)]) -> std::result::Result<Vec<deadpool_postgres::tokio_postgres::Row>, Box<dyn std::error::Error>> {
    let connection = pool.get().await?;
    let statement = connection.prepare_cached(query).await?;
    let rows = connection.query(&statement, params).await?;
    return Ok(rows);
}
