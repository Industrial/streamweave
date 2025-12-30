use crate::input::Input;
use crate::port::PortList;
use async_trait::async_trait;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

/// Helper trait for providing default port types for Consumers.
///
/// This trait provides default implementations of the `InputPorts` associated type.
/// All Consumers automatically get `InputPorts = (Self::Input,)` unless they
/// explicitly override it.
pub trait ConsumerPorts: Consumer
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The default input port tuple type (single port with the consumer's input type).
  type DefaultInputPorts: PortList;
}

/// Blanket implementation: all Consumers get default single-port input.
impl<C> ConsumerPorts for C
where
  C: Consumer,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
{
  type DefaultInputPorts = (C::Input,);
}

/// Configuration for a consumer component.
///
/// This struct holds configuration options that control how a consumer
/// behaves, including error handling strategy and component naming.
#[derive(Debug, Clone)]
pub struct ConsumerConfig<T: std::fmt::Debug + Clone + Send + Sync> {
  /// The error handling strategy to use when processing items.
  pub error_strategy: ErrorStrategy<T>,
  /// The name of this consumer component.
  pub name: String,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Default for ConsumerConfig<T> {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: String::new(),
    }
  }
}

/// Trait for components that consume data streams.
///
/// Consumers are the end point of a pipeline. They receive processed items
/// and typically write them to a destination (file, database, console, etc.)
/// or perform some final action.
///
/// # Example
///
/// ```rust
/// use streamweave::prelude::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut consumer = VecConsumer::new();
/// let stream = futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
/// consumer.consume(stream).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Implementations
///
/// Common consumer implementations include:
/// - `VecConsumer` - Collects items into a vector
/// - `FileConsumer` - Writes data to files
/// - `KafkaConsumer` - Produces to Kafka topics
/// - `ConsoleConsumer` - Prints items to console
#[async_trait]
pub trait Consumer: Input
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The input port tuple type for this consumer.
  ///
  /// This associated type specifies the port tuple that represents this consumer's
  /// inputs in the graph API. By default, consumers have a single input port
  /// containing their input type: `(Self::Input,)`.
  ///
  /// For multi-port consumers, override this type to specify a tuple with multiple
  /// input types, e.g., `(i32, String)` for two inputs.
  type InputPorts: PortList;

  /// Consumes a stream of items.
  ///
  /// This method is called by the pipeline to process the final stream.
  /// The consumer should handle all items in the stream and perform the
  /// appropriate action (write to file, send to database, etc.).
  ///
  /// # Arguments
  ///
  /// * `stream` - The stream of items to consume
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::prelude::*;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut consumer = VecConsumer::new();
  /// let stream = futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
  /// consumer.consume(stream).await?;
  /// # Ok(())
  /// # }
  /// ```
  async fn consume(&mut self, stream: Self::InputStream);

  /// Creates a new consumer instance with the given configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The configuration to apply to the consumer.
  #[must_use]
  fn with_config(&self, config: ConsumerConfig<Self::Input>) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  /// Sets the configuration for this consumer.
  ///
  /// # Arguments
  ///
  /// * `config` - The configuration to set.
  fn set_config(&mut self, config: ConsumerConfig<Self::Input>) {
    self.set_config_impl(config);
  }

  /// Returns a reference to the consumer's configuration.
  fn config(&self) -> &ConsumerConfig<Self::Input> {
    self.get_config_impl()
  }

  /// Returns a mutable reference to the consumer's configuration.
  fn config_mut(&mut self) -> &mut ConsumerConfig<Self::Input> {
    self.get_config_mut_impl()
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  #[must_use]
  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    self.config_mut().name = name.clone();
    self
  }

  /// Handles an error according to the consumer's error strategy.
  ///
  /// # Arguments
  ///
  /// * `error` - The error that occurred.
  ///
  /// # Returns
  ///
  /// The action to take based on the error strategy.
  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match &self.config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  /// Returns information about this consumer component.
  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config().name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  /// Creates an error context for the given item.
  ///
  /// # Arguments
  ///
  /// * `item` - The item that caused the error, if any.
  ///
  /// # Returns
  ///
  /// An error context containing information about when and where the error occurred.
  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  /// Internal implementation for setting configuration.
  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>);
  /// Internal implementation for getting configuration.
  fn get_config_impl(&self) -> &ConsumerConfig<Self::Input>;
  /// Internal implementation for getting mutable configuration.
  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input>;
}
