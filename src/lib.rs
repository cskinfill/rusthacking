pub mod models;

use models::{Service, Repository, RepoError};
use tracing::{info, instrument};

use sqlx::{ Pool, Sqlite};


#[derive(Debug)]
pub struct SqlRepo {
    pool: Pool<Sqlite>,
}

impl SqlRepo {
    pub fn new(pool: Pool<Sqlite>) -> Result<Self, RepoError> {
        Ok(Self { pool })
    }
}

impl Repository for SqlRepo {
    #[instrument]
    async fn services(&self)
        -> Result<Vec<Service>, RepoError> {
        sqlx::query_as::<_,Service>("SELECT * FROM services")
        .fetch_all(&self.pool)
        .await
        .map_err(|_| -> RepoError {RepoError::ServerError})
    }
    #[instrument]
    async fn service(
        &self,
        id: u32,
    ) -> Result<Service, RepoError> {
        sqlx::query_as::<_,Service>("SELECT * FROM services WHERE id=?")
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| -> RepoError {RepoError::ServerError})    
    }
}

#[derive(Debug,Clone)]
pub struct InMemoryRepo {
    _data: Vec<Service>,
}

impl InMemoryRepo {
    pub fn new(_data: Vec<Service>) -> Self { Self { _data } }
}

impl Repository for InMemoryRepo {
    #[instrument]
    async fn services(&self) -> Result<Vec<Service>, RepoError> {
        info!("In services");
        Ok(self._data.clone())
    }

    #[instrument]
    async fn service(&self, id:u32) -> Result<Service, RepoError> {
        info!("In service");
        self._data.iter().find(|s| s.id == id ).cloned()
        .ok_or(RepoError::Missing)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*; // Assuming `SqlRepo` is defined in the same module

    use sqlx::{Sqlite, SqlitePool};
    use tokio_test::assert_ok;
    use tracing::{debug,Level};
    use tracing_subscriber::fmt::format::FmtSpan;

    #[tokio::test]
    async fn create_and_query() {
        tracing_subscriber::fmt()        
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(Level::TRACE)
        .init();
        // Set up a temporary in-memory SQLite database for testing
        let database_url = "services.db";
        let pool = create_pool(database_url).await.unwrap();

        // Create a SqlRepo instance for testing
        let repo = SqlRepo::new(pool).unwrap();
        let test = repo.service(1).await;
        // Perform a simple test query
        debug!("test is: {:?}", test);
        assert_ok!(test); // Replace with expected value

        let test2 = repo.services().await;
        debug!("test2 is {:?}", test2);
    }

    // Add more tests for different methods and scenarios

    // Helper function to create a pool for testing
    async fn create_pool(database_url: &str) -> Result<Pool<Sqlite>, Box<dyn Error>> {
        // Create a migration directory if needed
        let migrations_dir = std::path::PathBuf::from("migrations");
        tokio::fs::create_dir_all(&migrations_dir).await?;

        // Establish the database connection pool
        Ok(SqlitePool::connect(database_url).await?)
    }
}
