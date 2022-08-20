use std::{sync::Arc};

use async_trait::async_trait;
use deadpool_postgres::{Pool, Transaction};
use scrypt::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Scrypt
};

use crate::{db::{query_one, DbResult}, files::paths_manager::PathManager};

pub struct UserDto {
    pub id: i64,
    pub name: String
}

pub enum ErrorVerifyPassword {
    UserNotFound,
    PasswordHasError(scrypt::password_hash::Error)
}

#[async_trait(?Send)] 
pub(crate) trait UserRepository {
    async fn verify_password(&self, user_name: &str, password: &str) -> Result<(), ErrorVerifyPassword>;
    async fn fetch_user(&self, user_name: &str) -> DbResult<UserDto>;
    async fn register_user(&self, user_name: &str, password: &str) -> Result<UserDto, ErrorRegisterUser>;
}

pub(crate) struct UserRepositoryImpl {
    pool: Arc<Pool>,
    path_manager: Arc<dyn PathManager>,
}

pub(crate) fn user_repository_new(pool: Arc<Pool>, path_manager: Arc<dyn PathManager>) -> impl UserRepository {
    UserRepositoryImpl {
        pool,
        path_manager
    }
}

#[async_trait(?Send)] 
impl UserRepository for UserRepositoryImpl {
    async fn verify_password(&self, user_name: &str, password: &str) -> Result<(), ErrorVerifyPassword> {
        let sql = r#"select ap.hash from authentication_password ap
        inner join authentication a on a.auth_password_id  = ap.id 
        inner join users u on u.id  = a.user_id 
        where u."name" = $1"#;
        if let Ok(row) = query_one(&self.pool, sql, &[&user_name]).await {
            let hash: String = row.get(0);
            return verify_hash(password,&hash).map_err(|e| ErrorVerifyPassword::PasswordHasError(e));
        }
        Err(ErrorVerifyPassword::UserNotFound)
    }

    async fn fetch_user(&self, user_name: &str) -> DbResult<UserDto> {
        let sql = r#"select u.id, u."name" from users u where u."name" = $1"#;
        let row = query_one(&self.pool, sql, &[&user_name]).await?;
        Ok(UserDto {
            id: row.get(0),
            name: user_name.to_owned()
        })
    }

    async fn register_user(&self, user_name: &str, password: &str) -> Result<UserDto, ErrorRegisterUser> {
        let connection = &mut self.pool.get().await.map_err(|_| ErrorRegisterUser::RegistrationFailed)?;
        let transaction = connection.transaction().await.map_err(|_| ErrorRegisterUser::RegistrationFailed)?;
        
        match self.initialize_user(&transaction, user_name, password).await {
            Ok(user) => {
                transaction.commit().await.map_err(|_| ErrorRegisterUser::RegistrationFailed)?;
                Ok(user)
            },
            Err(_) => {
                transaction.rollback().await.map_err(|_| ErrorRegisterUser::RegistrationFailed)?;
                Err(ErrorRegisterUser::RegistrationFailed)
            }
        }
    }
}

impl UserRepositoryImpl {
    async fn initialize_user(&self, transaction: &Transaction<'_>, user_name: &str, password: &str) -> Result<UserDto, Box<dyn std::error::Error>> {
        let insert_user_sql = "insert into users (name) values ($1) RETURNING id";
        let insert_password_sql = "insert into authentication_password (hash) values ($1) RETURNING id";
        let insert_auth_sql = "insert into authentication (user_id, auth_type_id, auth_password_id) values ($1, 1, $2)";
        let insert_file_root = r#"insert into file_nodes (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size)
        values (0, $1, 'ROOT', null, 0, $2, 'inode/directory', $3, 0)"#;
    
        let row = transaction.query_one(insert_user_sql, &[&user_name]).await?;
        let user_id: i64 = row.get(0);
        let password_hash = hash_password(password)?;
        let row = transaction.query_one(insert_password_sql, &[&password_hash]).await?;
        let password_id: i64 = row.get(0);
        transaction.execute(&format!("create sequence file_nodes_user_{} as bigint increment by 1 minvalue 1 NO MAXVALUE no cycle owned by file_nodes.id", user_id), &[]).await?;
        let user_files_root = &self.path_manager.get_top_save_folder(user_id);
        let user_files_root_str = user_files_root.as_os_str().to_str().unwrap();
        transaction.execute(insert_file_root, &[&user_id, &user_files_root_str, &chrono::Utc::now()]).await?;
        if let Ok(1) = transaction.execute(insert_auth_sql, &[&user_id, &password_id]).await {
            if let Ok(_) = &self.path_manager.init_user_fs(user_id) {
                return Ok(UserDto {
                    id: user_id,
                    name: user_name.to_owned()
                });
            }
        }
        Err("Unable to register user".into())
    }
}

pub enum ErrorRegisterUser {
    RegistrationFailed,
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
