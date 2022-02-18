use std::{error::Error, fmt::Display};

use actix_web::web;
use deadpool_postgres::Pool;
use scrypt::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Scrypt
};

use crate::db::{query_one, DbResult};

pub struct UserDto<'a> {
    pub id: i64,
    pub name: &'a str //Cow<'a, str>,
}

pub async fn verify_password(pool: &web::Data<Pool>, user_name: &str, password: &str) -> Result<(), Box<dyn Error>> {
    let sql = r#"select ap.hash from authentication_password ap
	inner join authentication a on a.auth_password_id  = ap.id 
	inner join users u on u.id  = a.user_id 
	where u."name" = $1"#;
    let row = query_one(pool, sql, &[&user_name]).await?;
    let hash: String = row.get(0);
    return verify_hash(password,&hash).map_err(|e| e.into());
}

pub async fn fetch_user<'a>(pool: &web::Data<Pool>, user_name: &'a str) -> DbResult<UserDto<'a>> {
    let sql = r#"select u.id, u."name" from users u where u."name" = $1"#;
    let row = query_one(pool, sql, &[&user_name]).await?;
    Ok(UserDto {
        id: row.get(0),
        name: user_name
    })
}

#[derive(Debug)]
struct UserRegistrationFailed {}

impl Display for UserRegistrationFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("User registration failed")
    }
}

impl Error for UserRegistrationFailed {
}

pub async fn register_user<'a>(pool: &web::Data<Pool>, user_name: &'a str, password: &str) -> Result<UserDto<'a>, Box<dyn Error>> {
    let mut connection = pool.get().await?;
    let transaction = connection.transaction().await?;
    
    let insert_user_sql = "insert into users (name) values ($1) RETURNING id";
    let insert_password_sql = "insert into authentication_password (hash) values ($1) RETURNING id";
    let insert_auth_sql = "insert into authentication (user_id, auth_type_id, auth_password_id) values ($1, 1, $2)";
    
    if let Ok(row) = transaction.query_one(insert_user_sql, &[&user_name]).await {
        let user_id: i64 = row.get(0);
        let password_hash = hash_password(password)?;
        if let Ok(row) = transaction.query_one(insert_password_sql, &[&password_hash]).await {
            let password_id: i64 = row.get(0);
            if let Ok(1) = transaction.execute(insert_auth_sql, &[&user_id, &password_id]).await {
                transaction.commit().await?;
                return Ok(UserDto {
                    id: user_id,
                    name: user_name
                });
            }
        }
    }
    transaction.rollback().await?;
    Err(UserRegistrationFailed{}.into())
}


fn hash_password(password: &str) -> Result<String, scrypt::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Scrypt.hash_password(password.as_bytes(), &salt)?;
    return Ok(password_hash.to_string());
}

fn verify_hash(password: &str, password_hash: &str) -> Result<(), scrypt::password_hash::Error> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    return Scrypt.verify_password(password.as_bytes(), &parsed_hash);
}
