//! Prometheus/OpenTelemetry-compatible metrics for StreamWeave.
//!
//! Records operational metrics (errors, throughput) for production observability.
//! Use [`install_prometheus_recorder`] at startup to expose metrics for scraping.
//!
//! ## Auto-scaling
//!
//! External controllers (e.g. Kubernetes HPA) can use these metrics:
//!
//! - **Throughput:** [`record_items_in`] / [`record_items_out`] feed
//!   `streamweave_items_in_total` and `streamweave_items_out_total` counters.
//!   Compute rate (items/sec) in Prometheus for scaling decisions.
//! - **Cluster size:** [`record_shard_assignment`] sets `streamweave_shard_id` and
//!   `streamweave_total_shards` gauges when running sharded.
//! - **Backlog (optional):** [`record_backlog_size`] sets `streamweave_backlog_size` gauge.
//!   Use when a source has a known backlog (e.g. Kafka consumer lag, queue depth). The
//!   built-in scaler can scale out when backlog exceeds a threshold.
//!
//! # Example
//!
//! ```rust,no_run
//! use streamweave::metrics;
//!
//! // At startup, install the Prometheus recorder (spawns HTTP server on default port)
//! metrics::install_prometheus_recorder();
//!
//! // Or with custom address:
//! // metrics::install_prometheus_recorder_on("0.0.0.0:9090".parse().unwrap());
//! ```

use metrics::counter;

/// Installs the Prometheus recorder as the global metrics recorder.
///
/// Spawns an HTTP server (when the `http-listener` feature is enabled) that serves
/// Prometheus metrics at `GET /metrics` on the default address. Call once at startup.
///
/// If not called, metrics recording is a no-op (metrics are dropped).
pub fn install_prometheus_recorder() {
  use metrics_exporter_prometheus::PrometheusBuilder;
  PrometheusBuilder::new()
    .install()
    .expect("failed to install Prometheus recorder");
}

/// Installs the Prometheus recorder and serves metrics on the given address.
///
/// Use when you need to configure the listen address. Spawns an HTTP server
/// that serves Prometheus metrics at `GET /metrics`.
pub fn install_prometheus_recorder_on(addr: std::net::SocketAddr) {
  use metrics_exporter_prometheus::PrometheusBuilder;
  PrometheusBuilder::new()
    .with_http_listener(addr)
    .install()
    .expect("failed to install Prometheus recorder");
}

/// Records a node execution error for the `streamweave_errors_total` counter.
pub fn record_node_error(graph_id: &str, node_id: &str) {
  counter!(
    "streamweave_errors_total",
    "graph_id" => graph_id.to_string(),
    "node_id" => node_id.to_string()
  )
  .increment(1);
}

/// Records items received on an input port.
///
/// Use for throughput metrics; external scalers can derive items/sec from
/// `streamweave_items_in_total` rate in Prometheus.
pub fn record_items_in(graph_id: &str, node_id: &str, port: &str, count: u64) {
  counter!(
    "streamweave_items_in_total",
    "graph_id" => graph_id.to_string(),
    "node_id" => node_id.to_string(),
    "port" => port.to_string()
  )
  .increment(count);
}

/// Records items sent on an output port.
///
/// Use for throughput metrics; external scalers can derive items/sec from
/// `streamweave_items_out_total` rate in Prometheus.
pub fn record_items_out(graph_id: &str, node_id: &str, port: &str, count: u64) {
  counter!(
    "streamweave_items_out_total",
    "graph_id" => graph_id.to_string(),
    "node_id" => node_id.to_string(),
    "port" => port.to_string()
  )
  .increment(count);
}

/// Records the current shard assignment when running cluster-sharded.
///
/// Call when the graph has [`ShardConfig`](crate::graph::ShardConfig) set (e.g. after
/// `set_shard_config`). Gauges `streamweave_shard_id` and `streamweave_total_shards`
/// allow external controllers to observe cluster size for scaling decisions.
pub fn record_shard_assignment(shard_id: u32, total_shards: u32) {
  metrics::gauge!("streamweave_shard_id").set(shard_id as f64);
  metrics::gauge!("streamweave_total_shards").set(total_shards as f64);
}

/// Records the current backlog size for scaling decisions.
///
/// Optional: call from sources that know their backlog (e.g. Kafka consumer lag,
/// number of items waiting in an input queue). The built-in scaler can scale out when
/// backlog exceeds a configured threshold. Use `graph_id` and `node_id` (or shard
/// identity) so the scaler can aggregate per graph or per shard.
pub fn record_backlog_size(graph_id: &str, node_id: &str, size: u64) {
  metrics::gauge!(
    "streamweave_backlog_size",
    "graph_id" => graph_id.to_string(),
    "node_id" => node_id.to_string()
  )
  .set(size as f64);
}
