#![allow(dead_code)]
use lazy_static::lazy_static;
use prometheus::{
    CounterVec, Gauge, HistogramVec, Registry, register_counter_vec, register_gauge,
    register_histogram_vec,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref CONTAINERS_RUNNING: Gauge = register_gauge!(
        "virtualos_containers_running",
        "Number of containers currently in running state"
    )
    .unwrap();

    pub static ref CONTAINERS_CREATED: CounterVec = register_counter_vec!(
        "virtualos_containers_created_total",
        "Total number of containers created",
        &[],
    )
    .unwrap();

    pub static ref CONTAINER_STARTS: CounterVec = register_counter_vec!(
        "virtualos_container_starts_total",
        "Total number of container start operations",
        &["status"], // success/failure
    )
    .unwrap();

    pub static ref CONTAINER_START_DURATION: HistogramVec = register_histogram_vec!(
        "virtualos_container_starts_duration_seconds",
        "Time spent starting a container",
        &["status"],
        vec![0.01, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0],
    )
    .unwrap();

    pub static ref ENGINE_ERRORS: CounterVec = register_counter_vec!(
        "virtualos_errors_total",
        "Total number of errors",
        &["operation"],
    )
    .unwrap();
}

/// Initialise metrics (call once at startup).
pub fn init_metrics() {
    // The lazy_static initialises on first use, but we can force it.
    let _ = &*CONTAINERS_RUNNING;
    // Optionally register with the default registry; prometheus crate uses default.
}
