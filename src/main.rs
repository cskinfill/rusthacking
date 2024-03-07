use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use metrics_process::Collector;
use rusthacking::models::{RepoError, Repository, Service};
use rusthacking::SqlRepo;
use sqlx::SqlitePool;
use tracing::*;
use tracing_subscriber::fmt::format::FmtSpan;

use axum_prometheus::PrometheusMetricLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let database_url = "services.db";
    let pool = SqlitePool::connect_lazy(database_url);
    let service_repo = Arc::new(SqlRepo::new(pool.unwrap()).unwrap());

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let collector = Collector::default();

    let app = Router::new()
        .route("/", get(root))
        .route("/services", get(all_services))
        .route("/service/:id", get(service))
        .route("/metrics",
            get(move || {
                collector.collect();
                std::future::ready(metric_handle.render())
            }),
        )
        .route_layer(prometheus_layer)
        .with_state(service_repo);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await
}

#[instrument]
async fn root() -> &'static str {
    "Hello, World!"
}

#[instrument(skip(repo))]
async fn all_services<T: Repository>(
    State(repo): State<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    repo.services()
        .await
        .map(|ss| Json(ss))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[instrument(skip(repo))]
async fn service<T: Repository>(
    Path(id): Path<u32>,
    State(repo): State<Arc<T>>,
) -> Result<Json<Service>, StatusCode> {
    repo.service(id)
        .await
        .map(|s| Json(s))
        .map_err(|e| match e {
            RepoError::Missing => StatusCode::NOT_FOUND,
            RepoError::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
        })
}
