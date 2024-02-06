mod sqlite;

use std::{collections::HashMap, fmt, future::IntoFuture, sync::Arc};

use diesel::Queryable;
use metrics_process::Collector;
use tracing::*;
use axum::{
    extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router
};
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format::FmtSpan;

use axum_prometheus::PrometheusMetricLayer;

use deadpool_diesel::sqlite::{Manager, Pool};
use diesel::{Connection, prelude::*};
use diesel_async::SqliteConnection;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()        
    .with_span_events(FmtSpan::CLOSE)
    .with_max_level(Level::TRACE)
    .init();

    // let mut data = HashMap::new();
    // data.insert(1,Service{id: 1, name: "Locate Us".to_string(), description: "Awesomeness is HERE!".to_string(), versions: 3 });
    // data.insert(2,Service{id: 2, name: "Contact Us".to_string(), description: "How can I find you?!".to_string(), versions: 2});

    // let service_repo = Arc::new(InMemoryRepo::new(data.clone()));

    let database_path = "database.db"; // Replace with your SQLite database path
    let manager = Manager::new(database_path)?;
    let pool = Pool::new(manager, 16)?;

    let service_repo = Arc::new(SqliteRepo{pool:pool});

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let collector = Collector::default();
    collector.describe();
    
    let app = Router::new()
        .route("/services", get(all_services))
        .route("/service/:id", get(service))    
        .route("/metrics", get(move || {
            collector.collect();
            std::future::ready(metric_handle.render())
        }))
        .route_layer(prometheus_layer)
        .with_state(service_repo);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[instrument(skip(repo))]
async fn all_services(State(repo): State<Arc<impl Repository>>) -> Result<impl IntoResponse, StatusCode> {
    repo.services().await
    .map(|ss| Json(ss))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[instrument]
async fn service(Path(id): Path<usize>, State(repo): State<Arc<InMemoryRepo>>) -> Result<impl IntoResponse, StatusCode> {
    repo.service(id).await?.map(|s| Json(s)).ok_or(StatusCode::NOT_FOUND)
}

#[derive(Serialize, Clone, Debug, Deserialize, Queryable)]
struct Service {
    id: usize,
    name: String,
    description: String,
    versions: usize,
}

#[derive(Debug, Clone)]
struct RepoError;

impl From<RepoError> for StatusCode {
    fn from(value: RepoError) -> Self {
        StatusCode::NOT_FOUND
    }
}

trait Repository {
    async fn services(&self) -> Result<Vec<Service>, RepoError>;
    async fn service(&self, id:usize) -> Result<Option<Service>, RepoError>;
}

#[derive(Debug,Clone)]
struct InMemoryRepo {
    _data: HashMap<usize, Service>,
}

impl InMemoryRepo {
    fn new(_data: HashMap<usize, Service>) -> Self { Self { _data } }
}

impl Repository for InMemoryRepo {
    #[instrument]
    async fn services(&self) -> Result<Vec<Service>, RepoError> {
        info!("In services");
        Ok(self._data.values().cloned().collect())
    }

    #[instrument]
    async fn service(&self, id:usize) -> Result<Option<Service>, RepoError> {
        info!("In service");
        Ok(self._data.get(&id).cloned())
    }
}

struct SqliteRepo {
    pool:Pool,
}

impl Repository for SqliteRepo {
    async fn services(&self) -> Result<Vec<Service>, RepoError> {
        let connection = pool.get().await?;
    }

    async fn service(&self, id:usize) -> Result<Option<Service>, RepoError> {
        let connection = self.pool.get().await.map_err(|e| RepoError)?;
        let result = services.find(id)
            .first::<Service>(&connection)
            .await
            .map_err(|_| "Model not found")?;

        Ok(None)
    }
}