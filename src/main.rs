use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use clap::Parser;
use metrics_process::Collector;
use rusthacking::models::{RepoError, Repository, Service};
use rusthacking::SqlRepo;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;
use tracing::*;
use tracing_subscriber::fmt::format::FmtSpan;

use axum_prometheus::PrometheusMetricLayer;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)] // requires `derive` feature
#[command(term_width = 0)] // Just to make testing across clap features easier
struct Args {
    #[arg(short = 'd', value_hint = clap::ValueHint::DirPath)]
    database: std::path::PathBuf,
    #[arg(short = 'p')]
    port: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let args = Args::parse();

    let pool = SqlitePool::connect_lazy(args.database.to_str().unwrap()).unwrap();
    let service_repo = Arc::new(SqlRepo::new(pool.clone()).unwrap());

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let collector = Collector::default();

    let app = Router::new()
        .route("/", get(root))
        .route("/services", get(all_services))
        .route("/service/{id}", get(service))
        .route_layer(prometheus_layer)
        .route(
            "/metrics",
            get(move || {
                collector.collect();
                std::future::ready(metric_handle.render())
            }),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(service_repo);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port.unwrap_or(3000))).await.unwrap();
    info!("App running on port {}", args.port.unwrap_or(3000));
    axum::serve(listener, app)
        .with_graceful_shutdown(signal_handler())
        .await?;
    info!("Closing db pool");
    pool.close().await;
    Ok(())
}

async fn signal_handler() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("Shutdown signal received, starting graceful shutdown...");
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
