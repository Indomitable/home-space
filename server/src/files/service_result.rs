use crate::db::DbError;

pub(crate) type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Debug)]
pub(crate) enum ServiceError {
    DbError(DbError),
    IOError(std::io::Error),
    Other(String, String)
}

impl From<DbError> for ServiceError {
    fn from(e: DbError) -> Self {
        Self::DbError(e)
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

impl ServiceError {
    pub(crate) fn new(service_name: &str, message: &str) -> Self {
        ServiceError::Other(service_name.to_string(), message.to_string())
    }
}
