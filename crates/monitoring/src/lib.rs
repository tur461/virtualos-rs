use anyhow::Result;
use axum::{Router, routing::get};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;

/// Start an HTTP server that exposes Prometheus metrics.
/// Binds to the given address, e.g., "0.0.0.0:9090".
pub async fn serve_metrics(addr: SocketAddr) -> Result<()> {
    let app = Router::new().route("/metrics", get(metrics_handler));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Metrics server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::default_registry().gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
