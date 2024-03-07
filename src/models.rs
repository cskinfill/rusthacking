use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use thiserror::Error;

#[derive(Serialize, Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, FromRow)]
pub struct Service {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub versions: u32,
}

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("data store disconnected")]
    ServerError,
    #[error("no element")]
    Missing,
}

pub trait Repository {
    fn services(&self)
        -> impl std::future::Future<Output = Result<Vec<Service>, RepoError>> + Send;
    fn service(
        &self,
        id: u32,
    ) -> impl std::future::Future<Output = Result<Service, RepoError>> + Send;
}

impl<T: Repository> Repository for &T {
    fn services(&self)
        -> impl std::future::Future<Output = Result<Vec<Service>, RepoError>> + Send {
        (**self).services()
    }

    fn service(
        &self,
        id: u32,
    ) -> impl std::future::Future<Output = Result<Service, RepoError>> + Send {
        (**self).service(id)
    }
}

impl<T: Repository> Repository for &mut T {
    fn services(&self)
        -> impl std::future::Future<Output = Result<Vec<Service>, RepoError>> + Send {
        (**self).services()
    }

    fn service(
        &self,
        id: u32,
    ) -> impl std::future::Future<Output = Result<Service, RepoError>> + Send {
        (**self).service(id)
    }
}

impl<T: Repository> Repository for Box<T> {
    fn services(&self)
        -> impl std::future::Future<Output = Result<Vec<Service>, RepoError>> + Send {
        (**self).services()
    }

    fn service(
        &self,
        id: u32,
    ) -> impl std::future::Future<Output = Result<Service, RepoError>> + Send {
        (**self).service(id)
    }
}
