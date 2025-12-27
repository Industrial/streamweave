//! # OpenTelemetry Integration
//!
//! This module provides OpenTelemetry integration for StreamWeave pipelines,
//! enabling distributed tracing and metrics export via OTLP.
//!
//! ## Features
//!
//! - **Trace Spans**: Automatic span creation for pipeline stages
//! - **Metrics Export**: Export metrics via OTLP (OpenTelemetry Protocol)
//! - **Context Propagation**: Trace context propagation through pipelines
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::metrics::opentelemetry::OpenTelemetryConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OpenTelemetryConfig::default()
//!     .with_service_name("my-pipeline")
//!     .with_otlp_endpoint("http://localhost:4317");
//!
//! let tracer = config.init_tracer().await?;
//! let meter = config.init_meter().await?;
//!
//! // Use tracer and meter with your pipeline
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "opentelemetry")]
use opentelemetry::{
  KeyValue, global,
  trace::{Tracer, TracerProvider},
};
#[cfg(feature = "opentelemetry")]
use opentelemetry_sdk::{
  Resource,
  metrics::SdkMeterProvider,
  trace::{SdkTracer, SdkTracerProvider, TraceError},
};
// MetricsError type alias - check actual location in opentelemetry 0.31

/// Configuration for OpenTelemetry integration.
///
/// This configuration allows you to set up OpenTelemetry tracing and metrics
/// export for StreamWeave pipelines.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::metrics::opentelemetry::OpenTelemetryConfig;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = OpenTelemetryConfig::default()
///     .with_service_name("my-pipeline")
///     .with_otlp_endpoint("http://localhost:4317");
///
/// let tracer = config.init_tracer().await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "opentelemetry")]
#[derive(Debug, Clone)]
pub struct OpenTelemetryConfig {
  /// Service name for traces and metrics.
  service_name: String,
  /// OTLP endpoint URL (default: http://localhost:4317).
  otlp_endpoint: String,
  /// Service version (optional).
  service_version: Option<String>,
  /// Additional resource attributes.
  resource_attributes: Vec<KeyValue>,
}

#[cfg(feature = "opentelemetry")]
impl Default for OpenTelemetryConfig {
  fn default() -> Self {
    Self {
      service_name: "streamweave-pipeline".to_string(),
      otlp_endpoint: "http://localhost:4317".to_string(),
      service_version: None,
      resource_attributes: Vec::new(),
    }
  }
}

#[cfg(feature = "opentelemetry")]
impl OpenTelemetryConfig {
  /// Creates a new OpenTelemetry configuration with default values.
  ///
  /// # Returns
  ///
  /// A new `OpenTelemetryConfig` instance.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the service name.
  ///
  /// # Arguments
  ///
  /// * `name` - The service name to use in traces and metrics.
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  #[must_use]
  pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
    self.service_name = name.into();
    self
  }

  /// Sets the OTLP endpoint URL.
  ///
  /// # Arguments
  ///
  /// * `endpoint` - The OTLP endpoint URL (e.g., "http://localhost:4317").
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  #[must_use]
  pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
    self.otlp_endpoint = endpoint.into();
    self
  }

  /// Sets the service version.
  ///
  /// # Arguments
  ///
  /// * `version` - The service version string.
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  #[must_use]
  pub fn with_service_version(mut self, version: impl Into<String>) -> Self {
    self.service_version = Some(version.into());
    self
  }

  /// Adds a resource attribute.
  ///
  /// # Arguments
  ///
  /// * `key` - The attribute key.
  /// * `value` - The attribute value.
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  #[must_use]
  pub fn with_resource_attribute(
    mut self,
    key: impl Into<String>,
    value: impl Into<String>,
  ) -> Self {
    let key_str: String = key.into();
    let value_str: String = value.into();
    self
      .resource_attributes
      .push(KeyValue::new(key_str.clone(), value_str.clone()));
    self
  }

  /// Initializes the OpenTelemetry tracer.
  ///
  /// This sets up the global tracer provider and returns a tracer instance.
  ///
  /// # Returns
  ///
  /// A `Tracer` instance, or an error if initialization fails.
  ///
  /// # Errors
  ///
  /// Returns an error if the tracer provider cannot be initialized.
  pub async fn init_tracer(&self) -> Result<SdkTracer, TraceError> {
    let service_name = self.service_name.clone();
    let mut resource_builder = Resource::builder().with_service_name(service_name);

    if let Some(version) = &self.service_version {
      let version = version.clone();
      resource_builder = resource_builder.with_attribute(KeyValue::new("service.version", version));
    }

    // Add custom attributes
    for attr in &self.resource_attributes {
      resource_builder = resource_builder.with_attribute(attr.clone());
    }

    let resource = resource_builder.build();

    use opentelemetry_otlp::SpanExporter;

    // Set endpoint via environment variable if custom endpoint is provided
    if self.otlp_endpoint != "http://localhost:4317" {
      unsafe {
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", &self.otlp_endpoint);
      }
    }

    let exporter = SpanExporter::builder()
      .with_tonic()
      .build()
      .map_err(|e| TraceError::Other(format!("Failed to create span exporter: {}", e).into()))?;

    let tracer_provider = SdkTracerProvider::builder()
      .with_batch_exporter(exporter)
      .with_resource(resource)
      .build();

    let tracer = tracer_provider.tracer("streamweave");
    global::set_tracer_provider(tracer_provider.clone());

    Ok(tracer)
  }

  /// Initializes the OpenTelemetry meter for metrics export.
  ///
  /// This sets up the global meter provider and returns a meter instance.
  ///
  /// # Returns
  ///
  /// A `Meter` instance, or an error if initialization fails.
  ///
  /// # Errors
  ///
  /// Returns an error if the meter provider cannot be initialized.
  pub async fn init_meter(
    &self,
  ) -> Result<opentelemetry::metrics::Meter, Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry::metrics::MeterProvider;

    let service_name = self.service_name.clone();
    let service_version = self.service_version.clone();
    let resource_attributes = self.resource_attributes.clone();

    let mut resource_builder = Resource::builder().with_service_name(service_name);

    if let Some(version) = &service_version {
      let version = version.clone();
      resource_builder = resource_builder.with_attribute(KeyValue::new("service.version", version));
    }

    // Add custom attributes
    for attr in &resource_attributes {
      resource_builder = resource_builder.with_attribute(attr.clone());
    }

    let resource = resource_builder.build();

    use opentelemetry_otlp::MetricExporter;

    // Set endpoint via environment variable if custom endpoint is provided
    if self.otlp_endpoint != "http://localhost:4317" {
      unsafe {
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", &self.otlp_endpoint);
      }
    }

    let exporter = MetricExporter::builder()
      .with_tonic()
      .build()
      .map_err(|e| format!("Failed to create metric exporter: {}", e))?;

    let meter_provider = SdkMeterProvider::builder()
      .with_periodic_exporter(exporter)
      .with_resource(resource)
      .build();

    let meter = meter_provider.meter("streamweave");
    global::set_meter_provider(meter_provider.clone());

    Ok(meter)
  }

  /// Gets the service name.
  ///
  /// # Returns
  ///
  /// The service name.
  #[must_use]
  pub fn service_name(&self) -> &str {
    &self.service_name
  }

  /// Gets the OTLP endpoint.
  ///
  /// # Returns
  ///
  /// The OTLP endpoint URL.
  #[must_use]
  pub fn otlp_endpoint(&self) -> &str {
    &self.otlp_endpoint
  }
}

/// Helper for creating trace spans for pipeline stages.
///
/// This utility helps create and manage OpenTelemetry spans for different
/// stages of pipeline execution (producer, transformer, consumer).
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::metrics::opentelemetry::PipelineTracer;
/// use opentelemetry::trace::Tracer;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let tracer = opentelemetry::global::tracer("my-pipeline");
/// let pipeline_tracer = PipelineTracer::new(&tracer, "my-pipeline");
///
/// // Create spans for pipeline stages
/// let producer_span = pipeline_tracer.producer_span("RangeProducer");
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "opentelemetry")]
pub struct PipelineTracer {
  tracer: SdkTracer,
  pipeline_name: String,
}

#[cfg(feature = "opentelemetry")]
impl PipelineTracer {
  /// Creates a new pipeline tracer.
  ///
  /// # Arguments
  ///
  /// * `tracer` - The OpenTelemetry tracer to use.
  /// * `pipeline_name` - The name of the pipeline.
  ///
  /// # Returns
  ///
  /// A new `PipelineTracer` instance.
  #[must_use]
  pub fn new(tracer: SdkTracer, pipeline_name: impl Into<String>) -> Self {
    Self {
      tracer,
      pipeline_name: pipeline_name.into(),
    }
  }

  /// Creates a span for a producer stage.
  ///
  /// # Arguments
  ///
  /// * `producer_name` - The name/type of the producer.
  ///
  /// # Returns
  ///
  /// A span builder that can be used to create the span.
  pub fn producer_span(&self, producer_name: &str) -> opentelemetry::trace::SpanBuilder {
    let producer_name = producer_name.to_string();
    self
      .tracer
      .span_builder(format!("producer.{}", producer_name))
      .with_kind(opentelemetry::trace::SpanKind::Producer)
      .with_attributes(vec![
        KeyValue::new("pipeline.name", self.pipeline_name.clone()),
        KeyValue::new("component.type", "producer"),
        KeyValue::new("component.name", producer_name),
      ])
  }

  /// Creates a span for a transformer stage.
  ///
  /// # Arguments
  ///
  /// * `transformer_name` - The name/type of the transformer.
  ///
  /// # Returns
  ///
  /// A span builder that can be used to create the span.
  pub fn transformer_span(&self, transformer_name: &str) -> opentelemetry::trace::SpanBuilder {
    let transformer_name = transformer_name.to_string();
    self
      .tracer
      .span_builder(format!("transformer.{}", transformer_name))
      .with_kind(opentelemetry::trace::SpanKind::Internal)
      .with_attributes(vec![
        KeyValue::new("pipeline.name", self.pipeline_name.clone()),
        KeyValue::new("component.type", "transformer"),
        KeyValue::new("component.name", transformer_name),
      ])
  }

  /// Creates a span for a consumer stage.
  ///
  /// # Arguments
  ///
  /// * `consumer_name` - The name/type of the consumer.
  ///
  /// # Returns
  ///
  /// A span builder that can be used to create the span.
  pub fn consumer_span(&self, consumer_name: &str) -> opentelemetry::trace::SpanBuilder {
    let consumer_name = consumer_name.to_string();
    self
      .tracer
      .span_builder(format!("consumer.{}", consumer_name))
      .with_kind(opentelemetry::trace::SpanKind::Consumer)
      .with_attributes(vec![
        KeyValue::new("pipeline.name", self.pipeline_name.clone()),
        KeyValue::new("component.type", "consumer"),
        KeyValue::new("component.name", consumer_name),
      ])
  }

  /// Creates a span for the entire pipeline execution.
  ///
  /// # Returns
  ///
  /// A span builder that can be used to create the span.
  pub fn pipeline_span(&self) -> opentelemetry::trace::SpanBuilder {
    self
      .tracer
      .span_builder(format!("pipeline.{}", self.pipeline_name))
      .with_kind(opentelemetry::trace::SpanKind::Internal)
      .with_attributes(vec![KeyValue::new(
        "pipeline.name",
        self.pipeline_name.clone(),
      )])
  }
}

/// Helper for exporting metrics via OpenTelemetry.
///
/// This utility helps export StreamWeave metrics to OpenTelemetry-compatible
/// backends via OTLP.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::metrics::opentelemetry::MetricsExporter;
/// use streamweave::metrics::PipelineMetrics;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let meter = opentelemetry::global::meter("my-pipeline");
/// let exporter = MetricsExporter::new(&meter, "my-pipeline");
///
/// let metrics = PipelineMetrics::new("my-pipeline");
/// exporter.export_metrics(&metrics).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "opentelemetry")]
pub struct MetricsExporter {
  #[allow(dead_code)] // Kept for potential future use
  meter: opentelemetry::metrics::Meter,
  pipeline_name: String,
  throughput_counter: opentelemetry::metrics::Counter<u64>,
  latency_histogram: opentelemetry::metrics::Histogram<f64>,
  error_counter: opentelemetry::metrics::Counter<u64>,
}

#[cfg(feature = "opentelemetry")]
impl MetricsExporter {
  /// Creates a new metrics exporter.
  ///
  /// # Arguments
  ///
  /// * `meter` - The OpenTelemetry meter to use.
  /// * `pipeline_name` - The name of the pipeline.
  ///
  /// # Returns
  ///
  /// A new `MetricsExporter` instance.
  pub fn new(meter: &opentelemetry::metrics::Meter, pipeline_name: impl Into<String>) -> Self {
    let pipeline_name = pipeline_name.into();
    let throughput_counter = meter
      .u64_counter("streamweave.items_processed")
      .with_description("Number of items processed by the pipeline")
      .build();
    let latency_histogram = meter
      .f64_histogram("streamweave.latency_seconds")
      .with_description("Processing latency in seconds")
      .build();
    let error_counter = meter
      .u64_counter("streamweave.errors_total")
      .with_description("Total number of errors encountered")
      .build();

    Self {
      meter: meter.clone(),
      pipeline_name,
      throughput_counter,
      latency_histogram,
      error_counter,
    }
  }

  /// Exports metrics from a `PipelineMetrics` instance.
  ///
  /// # Arguments
  ///
  /// * `metrics` - The pipeline metrics to export.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::metrics::opentelemetry::MetricsExporter;
  /// use streamweave::metrics::PipelineMetrics;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let meter = opentelemetry::global::meter("my-pipeline");
  /// let exporter = MetricsExporter::new(&meter, "my-pipeline");
  ///
  /// let metrics = PipelineMetrics::new("my-pipeline");
  /// exporter.export_metrics(&metrics).await;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn export_metrics(&self, metrics: &crate::metrics::types::PipelineMetrics) {
    let attributes = vec![KeyValue::new("pipeline.name", self.pipeline_name.clone())];

    // Export throughput
    let throughput = metrics.throughput().items_processed();
    self.throughput_counter.add(throughput, &attributes);

    // Export latency (average)
    if let Some(avg_latency) = metrics.latency().latency_avg() {
      let latency_seconds = avg_latency.as_secs_f64();
      self.latency_histogram.record(latency_seconds, &attributes);
    }

    // Export error count
    let error_count = metrics.errors().total_errors();
    self.error_counter.add(error_count, &attributes);
  }
}

#[cfg(not(all(feature = "opentelemetry")))]
// Placeholder implementations for when OpenTelemetry is not enabled
pub struct OpenTelemetryConfig;

#[cfg(not(all(feature = "opentelemetry")))]
impl Default for OpenTelemetryConfig {
  fn default() -> Self {
    Self
  }
}

#[cfg(not(all(feature = "opentelemetry")))]
impl OpenTelemetryConfig {
  #[must_use]
  pub fn new() -> Self {
    Self
  }
}
