use crate::output::Output;
use crate::port::PortList;
use async_trait::async_trait;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

/// Configuration for a producer component.
///
/// This struct holds configuration options that control how a producer
/// behaves, including error handling strategy and component naming.
#[derive(Debug, Clone)]
pub struct ProducerConfig<T: std::fmt::Debug + Clone + Send + Sync> {
  /// The error handling strategy to use when producing items.
  pub error_strategy: ErrorStrategy<T>,
  /// Optional name for identifying this producer in logs and metrics.
  pub name: Option<String>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Default for ProducerConfig<T> {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: None,
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> ProducerConfig<T> {
  /// Sets the error handling strategy for this producer configuration.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.name = Some(name);
    self
  }

  /// Returns the current error handling strategy.
  pub fn error_strategy(&self) -> ErrorStrategy<T> {
    self.error_strategy.clone()
  }

  /// Returns the current name, if set.
  pub fn name(&self) -> Option<String> {
    self.name.clone()
  }
}

/// Helper trait for providing default port types for Producers.
///
/// This trait provides default implementations of the `OutputPorts` associated type.
/// All Producers automatically get `OutputPorts = (Self::Output,)` unless they
/// explicitly override it.
pub trait ProducerPorts: Producer
where
  Self::Output: std::fmt::Debug + Clone + Send + Sync,
{
  /// The default output port tuple type (single port with the producer's output type).
  type DefaultOutputPorts: crate::port::PortList;
}

/// Blanket implementation: all Producers get default single-port output.
impl<P> ProducerPorts for P
where
  P: Producer,
  P::Output: std::fmt::Debug + Clone + Send + Sync,
{
  type DefaultOutputPorts = (P::Output,);
}

/// Trait for components that produce data streams.
///
/// Producers generate items that flow through the pipeline. They are the
/// starting point of any StreamWeave pipeline.
///
/// # Example
///
/// ```rust
/// use streamweave::prelude::*;
///
/// let mut producer = ArrayProducer::new(vec![1, 2, 3, 4, 5]);
/// let stream = producer.produce();
/// // Stream yields: 1, 2, 3, 4, 5
/// ```
///
/// # Implementations
///
/// Common producer implementations include:
/// - `ArrayProducer` - Produces items from a vector
/// - `FileProducer` - Reads data from files
/// - `KafkaProducer` - Consumes from Kafka topics
/// - `DatabaseProducer` - Queries database tables
#[async_trait]
pub trait Producer: Output
where
  Self::Output: std::fmt::Debug + Clone + Send + Sync,
{
  /// The output port tuple type for this producer.
  ///
  /// This associated type specifies the port tuple that represents this producer's
  /// outputs in the graph API. By default, producers have a single output port
  /// containing their output type: `(Self::Output,)`.
  ///
  /// For multi-port producers, override this type to specify a tuple with multiple
  /// output types, e.g., `(i32, String)` for two outputs.
  ///
  /// **Note**: If you don't specify this type, use `ProducerNode::from_producer()`
  /// which will automatically infer the port types using `ProducerPorts::DefaultOutputPorts`.
  type OutputPorts: PortList;

  /// Produces a stream of items.
  ///
  /// This method is called by the pipeline to generate the input stream.
  /// The returned stream will be consumed by transformers and eventually
  /// by consumers.
  ///
  /// # Returns
  ///
  /// A stream that yields items of type `Self::Output`.
  fn produce(&mut self) -> Self::OutputStream;

  /// Creates a new producer instance with the given configuration.
  ///
  /// This method clones the producer and applies the provided configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The `ProducerConfig` to apply.
  ///
  /// # Returns
  ///
  /// A new producer instance with the specified configuration.
  #[must_use]
  fn with_config(&self, config: ProducerConfig<Self::Output>) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  /// Sets the configuration for this producer.
  ///
  /// # Arguments
  ///
  /// * `config` - The new `ProducerConfig` to apply.
  fn set_config(&mut self, config: ProducerConfig<Self::Output>) {
    self.set_config_impl(config);
  }

  /// Returns a reference to the producer's configuration.
  ///
  /// # Returns
  ///
  /// A reference to the `ProducerConfig` for this producer.
  fn config(&self) -> &ProducerConfig<Self::Output> {
    self.get_config_impl()
  }

  /// Returns a mutable reference to the producer's configuration.
  ///
  /// # Returns
  ///
  /// A mutable reference to the `ProducerConfig` for this producer.
  fn config_mut(&mut self) -> &mut ProducerConfig<Self::Output> {
    self.get_config_mut_impl()
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  ///
  /// # Returns
  ///
  /// The producer instance with the updated name.
  #[must_use]
  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    let config = self.get_config_impl().clone();
    self.set_config(ProducerConfig {
      error_strategy: config.error_strategy,
      name: Some(name),
    });
    self
  }

  /// Handles an error that occurred during stream production.
  ///
  /// This method determines the appropriate `ErrorAction` based on the
  /// producer's configured `ErrorStrategy`.
  ///
  /// # Arguments
  ///
  /// * `error` - The `StreamError` that occurred.
  ///
  /// # Returns
  ///
  /// The `ErrorAction` to take in response to the error.
  fn handle_error(&self, error: &StreamError<Self::Output>) -> ErrorAction {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  /// Creates an error context for error reporting.
  ///
  /// This method constructs an `ErrorContext` with the current timestamp,
  /// the item that caused the error (if any), and component information.
  ///
  /// # Arguments
  ///
  /// * `item` - The item that caused the error, if available.
  ///
  /// # Returns
  ///
  /// An `ErrorContext` containing error details.
  fn create_error_context(&self, item: Option<Self::Output>) -> ErrorContext<Self::Output> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  /// Returns information about the component for error reporting.
  ///
  /// This includes the component's name and type.
  ///
  /// # Returns
  ///
  /// A `ComponentInfo` struct containing details about the producer.
  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  /// Sets the configuration implementation.
  ///
  /// This method must be implemented by each producer to store the configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The `ProducerConfig` to store.
  fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>);

  /// Returns a reference to the configuration implementation.
  ///
  /// This method must be implemented by each producer to return its stored configuration.
  ///
  /// # Returns
  ///
  /// A reference to the producer's `ProducerConfig`.
  fn get_config_impl(&self) -> &ProducerConfig<Self::Output>;

  /// Returns a mutable reference to the configuration implementation.
  ///
  /// This method must be implemented by each producer to return a mutable reference
  /// to its stored configuration.
  ///
  /// # Returns
  ///
  /// A mutable reference to the producer's `ProducerConfig`.
  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output>;
}
