use std::sync::Arc;

use async_trait::async_trait;
use deadpool_postgres::{Pool, Transaction};
use scrypt::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Scrypt
};

use crate::{db::{DbResult, DatabaseAccess}, files::paths_manager::PathManager};

pub struct UserDto {
    pub id: i64,
    pub name: String
}

pub enum ErrorVerifyPassword {
    UserNotFound,
    PasswordHasError(scrypt::password_hash::Error)
}

#[async_trait] 
pub(crate) trait UserRepository {
    async fn verify_password(&self, user_name: &str, password: &str) -> Result<(), ErrorVerifyPassword>;
    async fn fetch_user(&self, user_name: &str) -> DbResult<UserDto>;
    async fn register_user(&self, user_name: &str, password: &str) -> Result<UserDto, ErrorRegisterUser>;
}

pub(crate) struct UserRepositoryImpl {
    pool: Arc<Pool>,
    path_manager: Arc<dyn PathManager + Send + Sync>,
    db: Arc<dyn DatabaseAccess + Send + Sync>,
}

pub(crate) fn user_repository_new<PM, DA>(pool: Arc<Pool>, path_manager: Arc<PM>, db: Arc<DA>) -> impl UserRepository
where PM: PathManager + Send + Sync + 'static, 
      DA: DatabaseAccess + Send + Sync + 'static {
    UserRepositoryImpl {
        pool,
        path_manager,
        db
    }
}

#[async_trait] 
impl UserRepository for UserRepositoryImpl {
    async fn verify_password(&self, user_name: &str, password: &str) -> Result<(), ErrorVerifyPassword> {
        let sql = r#"select ap.hash from authentication_password ap
        inner join authentication a on a.auth_password_id  = ap.id 
        inner join users u on u.id  = a.user_id 
        where u."name" = $1"#;
        if let Ok(row) = self.db.query_one(&self.pool, sql, &[&user_name]).await {
            let hash: String = row.get(0);
            return verify_hash(password,&hash).map_err(|e| ErrorVerifyPassword::PasswordHasError(e));
        }
        Err(ErrorVerifyPassword::UserNotFound)
    }

    async fn fetch_user(&self, user_name: &str) -> DbResult<UserDto> {
        let sql = r#"select u.id, u."name" from users u where u."name" = $1"#;
        let row = self.db.query_one(&self.pool, sql, &[&user_name]).await?;
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
            Err(err) => {
                transaction.rollback().await.map_err(|_| ErrorRegisterUser::RegistrationFailed)?;
                log::error!("Error while registering user: {:?}", err);
                Err(ErrorRegisterUser::RegistrationFailed)
            }
        }
    }
}

impl UserRepositoryImpl {
    async fn initialize_user(&self, transaction: &Transaction<'_>, user_name: &str, password: &str) -> Result<UserDto, ErrorRegisterUser> {
        let insert_user_sql = "insert into users (name) values ($1) RETURNING id";
        let insert_password_sql = "insert into authentication_password (hash) values ($1) RETURNING id";
        let insert_auth_sql = "insert into authentication (user_id, auth_type_id, auth_password_id) values ($1, 1, $2)";
        let insert_file_root = r#"insert into file_nodes (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size, node_version)
        values (0, $1, 'ROOT', null, 0, $2, 'inode/directory', $3, 0, 1)"#;
    
        let row = transaction.query_one(insert_user_sql, &[&user_name]).await.map_err(|_| ErrorRegisterUser::InsertUserFailed)?;
        let user_id: i64 = row.get(0);
        let password_hash = hash_password(password).map_err(|_| ErrorRegisterUser::HashingPasswordFailed)?;
        let row = transaction.query_one(insert_password_sql, &[&password_hash]).await.map_err(|_| ErrorRegisterUser::InsertPasswordFailed)?;
        let password_id: i64 = row.get(0);
        transaction.execute(&format!("create sequence file_nodes_user_{} as bigint increment by 1 minvalue 1 NO MAXVALUE no cycle owned by file_nodes.id", user_id), &[])
            .await
            .map_err(|_| ErrorRegisterUser::CreateFileNodeSequenceFailed)?;
        transaction.execute(insert_file_root, &[&user_id, &"/", &chrono::Utc::now()]).await.map_err(|_| ErrorRegisterUser::InsertFileRootFailed)?;
        if let Ok(1) = transaction.execute(insert_auth_sql, &[&user_id, &password_id]).await {
            if let Ok(_) = self.path_manager.init_user_fs(user_id) {
                return Ok(UserDto {
                    id: user_id,
                    name: user_name.to_owned()
                });
            }
        }
        Err(ErrorRegisterUser::RegistrationFailed)
    }
}

#[derive(Debug)]
pub enum ErrorRegisterUser {
    RegistrationFailed,
    InsertUserFailed,
    HashingPasswordFailed,
    InsertPasswordFailed,
    CreateFileNodeSequenceFailed,
    InsertFileRootFailed,
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
