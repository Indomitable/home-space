use std::{env::{self, VarError}};

pub fn init_config() {
    let env = env::var("APP_ENVIRONMENT").or::<VarError>(Ok("dev".into())).unwrap();
    let config_path = format!(".env.{}", env);
    dotenv::from_filename(&config_path).unwrap();
}

pub fn get_listen_address() -> String {
    format!("{}:{}", env::var("SERVER_NAME").unwrap(), env::var("SERVER_PORT").unwrap())
}

pub fn get_host_url() -> String {
    format!("{}://{}", env::var("SERVER_SCHEMA").unwrap(), get_listen_address())
}

pub fn get_db_connection_url() -> String {
    let server = env::var("DB_SERVER_NAME").unwrap();
    let database = env::var("DB_SERVER_DATABASE").unwrap();
    let user_name = env::var("DB_SERVER_USER_NAME").unwrap();
    let password = env::var("DB_SERVER_PASSWORD").unwrap();
    format!("postgresql://{}:{}@{}/{}?connect_timeout=10&application_name=home-space", user_name, password, server, database)
}

pub fn get_jwt_secret() -> String {
    return env::var("JWT_TOKEN_SEC_KEY").unwrap();
}

pub fn get_files_location() -> String {
    return env::var("FILES_LOCATION").unwrap();
}

pub fn is_ssl_enabled() -> bool {
    return env::var("SSL_ENABLE").unwrap_or("0".to_owned()) == "1";
}

pub fn ssl_listen_address() -> String {
    format!("{}:{}", env::var("SERVER_NAME").unwrap(), env::var("SSL_PORT").unwrap())
}

pub fn ssl_private_key() -> String {
    return env::var("SSL_PRIVATE_KEY").unwrap();
}

pub fn ssl_chain_key() -> String {
    return env::var("SSL_CHAIN_KEY").unwrap();
}

pub fn is_prod() -> bool {
    return env::var("PROD").unwrap_or("1".to_owned()) == "1";
}
