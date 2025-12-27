use crate::port::PortList;
use crate::{input::Input, output::Output};
use async_trait::async_trait;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

/// Helper trait for providing default port types for Transformers.
///
/// This trait provides default implementations of the `InputPorts` and `OutputPorts` associated types.
/// All Transformers automatically get `InputPorts = (Self::Input,)` and `OutputPorts = (Self::Output,)`
/// unless they explicitly override them.
pub trait TransformerPorts: Transformer
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The default input port tuple type (single port with the transformer's input type).
  type DefaultInputPorts: PortList;
  /// The default output port tuple type (single port with the transformer's output type).
  type DefaultOutputPorts: PortList;
}

/// Blanket implementation: all Transformers get default single-port inputs and outputs.
impl<T> TransformerPorts for T
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
{
  type DefaultInputPorts = (T::Input,);
  type DefaultOutputPorts = (T::Output,);
}

/// Configuration for transformers, including error handling strategy and naming.
///
/// This struct holds configuration options that can be applied to any transformer,
/// allowing customization of error handling behavior and component identification.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformerConfig<T: std::fmt::Debug + Clone + Send + Sync> {
  /// The error handling strategy to use when errors occur.
  pub error_strategy: ErrorStrategy<T>,
  /// Optional name for identifying this transformer in logs and metrics.
  pub name: Option<String>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Default for TransformerConfig<T> {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: None,
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> TransformerConfig<T> {
  /// Sets the error handling strategy for this transformer configuration.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
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

/// Trait for components that transform data streams.
///
/// Transformers process items as they flow through the pipeline. They can
/// filter, map, aggregate, or perform any other transformation on stream items.
///
/// # Example
///
/// ```rust
/// use streamweave::prelude::*;
///
/// let mut transformer = MapTransformer::new(|x: i32| x * 2);
/// let input = futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
/// let output = transformer.transform(input);
/// // Output stream yields: 2, 4, 6
/// ```
///
/// # Implementations
///
/// Common transformer implementations include:
/// - `MapTransformer` - Transforms each item
/// - `FilterTransformer` - Filters items based on a predicate
/// - `BatchTransformer` - Groups items into batches
/// - `RateLimitTransformer` - Controls throughput
/// - `RetryTransformer` - Retries failed items
#[async_trait]
pub trait Transformer: Input + Output
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The input port tuple type for this transformer.
  ///
  /// This associated type specifies the port tuple that represents this transformer's
  /// inputs in the graph API. By default, transformers have a single input port
  /// containing their input type: `(Self::Input,)`.
  ///
  /// For multi-port transformers, override this type to specify a tuple with multiple
  /// input types, e.g., `(i32, String)` for two inputs.
  type InputPorts: PortList;

  /// The output port tuple type for this transformer.
  ///
  /// This associated type specifies the port tuple that represents this transformer's
  /// outputs in the graph API. By default, transformers have a single output port
  /// containing their output type: `(Self::Output,)`.
  ///
  /// For multi-port transformers, override this type to specify a tuple with multiple
  /// output types, e.g., `(i32, String)` for two outputs.
  type OutputPorts: PortList;

  /// Transforms a stream of input items into a stream of output items.
  ///
  /// This method is called by the pipeline to process items. The transformer
  /// receives items from the previous component and produces transformed items
  /// for the next component.
  ///
  /// # Arguments
  ///
  /// * `input` - The input stream to transform
  ///
  /// # Returns
  ///
  /// A stream that yields transformed items of type `Self::Output`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::prelude::*;
  ///
  /// let mut transformer = MapTransformer::new(|x: i32| x * 2);
  /// let input = futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
  /// let output = transformer.transform(input);
  /// ```
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream;

  /// Creates a new transformer instance with the given configuration.
  ///
  /// This method clones the transformer and applies the provided configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The `TransformerConfig` to apply.
  ///
  /// # Returns
  ///
  /// A new transformer instance with the specified configuration.
  #[must_use]
  fn with_config(&self, config: TransformerConfig<Self::Input>) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  /// Sets the configuration for this transformer.
  ///
  /// # Arguments
  ///
  /// * `config` - The new `TransformerConfig` to apply.
  fn set_config(&mut self, config: TransformerConfig<Self::Input>) {
    self.set_config_impl(config);
  }

  /// Returns a reference to the transformer's configuration.
  ///
  /// # Returns
  ///
  /// A reference to the `TransformerConfig` for this transformer.
  fn config(&self) -> &TransformerConfig<Self::Input> {
    self.get_config_impl()
  }

  /// Returns a mutable reference to the transformer's configuration.
  ///
  /// # Returns
  ///
  /// A mutable reference to the `TransformerConfig` for this transformer.
  fn config_mut(&mut self) -> &mut TransformerConfig<Self::Input> {
    self.get_config_mut_impl()
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  ///
  /// # Returns
  ///
  /// The transformer instance with the updated name.
  #[must_use]
  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    let config = self.get_config_impl().clone();
    self.set_config(TransformerConfig {
      error_strategy: config.error_strategy,
      name: Some(name),
    });
    self
  }

  /// Handles an error that occurred during stream processing.
  ///
  /// This method determines the appropriate `ErrorAction` based on the
  /// transformer's configured `ErrorStrategy`.
  ///
  /// # Arguments
  ///
  /// * `error` - The `StreamError` that occurred.
  ///
  /// # Returns
  ///
  /// The `ErrorAction` to take in response to the error.
  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
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
  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
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
  /// A `ComponentInfo` struct containing details about the transformer.
  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  /// Sets the configuration implementation.
  ///
  /// This method must be implemented by each transformer to store the configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The `TransformerConfig` to store.
  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>);

  /// Returns a reference to the configuration implementation.
  ///
  /// This method must be implemented by each transformer to return its stored configuration.
  ///
  /// # Returns
  ///
  /// A reference to the transformer's `TransformerConfig`.
  fn get_config_impl(&self) -> &TransformerConfig<Self::Input>;

  /// Returns a mutable reference to the configuration implementation.
  ///
  /// This method must be implemented by each transformer to return a mutable reference
  /// to its stored configuration.
  ///
  /// # Returns
  ///
  /// A mutable reference to the transformer's `TransformerConfig`.
  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use std::pin::Pin;
  use streamweave_error::{ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use tokio_stream::Stream;

  // Test error type
  #[derive(Debug)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  // Test transformer that doubles the input
  #[derive(Clone, Debug)]
  struct TestTransformer<T: std::fmt::Debug + Clone + Send + Sync> {
    config: TransformerConfig<T>,
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync> TestTransformer<T> {
    fn new() -> Self {
      Self {
        config: TransformerConfig::default(),
      }
    }
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TestTransformer<T> {
    type Input = T;
    type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TestTransformer<T> {
    type Output = T;
    type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
  }

  #[async_trait]
  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TestTransformer<T> {
    type InputPorts = (T,);
    type OutputPorts = (T,);

    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      Box::pin(input)
    }

    fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
      &self.config
    }

    fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
      &mut self.config
    }
  }

  #[tokio::test]
  async fn test_transformer() {
    let mut transformer = TestTransformer::<i32>::new();
    let input = futures::stream::iter(vec![1, 2, 3]);
    let output = transformer.transform(Box::pin(input));
    let result: Vec<i32> = output.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[test]
  fn test_transformer_config() {
    let transformer = TestTransformer::<i32>::new().with_config(
      TransformerConfig::default()
        .with_name("test_transformer".to_string())
        .with_error_strategy(ErrorStrategy::Skip),
    );

    assert_eq!(
      transformer.config().name(),
      Some("test_transformer".to_string())
    );
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_transformer_error_handling() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: transformer.component_info().name,
        component_type: transformer.component_info().type_name,
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));
  }

  #[tokio::test]
  async fn test_empty_input() {
    let mut transformer = TestTransformer::<i32>::new();
    let input = futures::stream::iter(Vec::<i32>::new());
    let output = transformer.transform(Box::pin(input));
    let result: Vec<i32> = output.collect().await;
    assert_eq!(result, Vec::<i32>::new());
  }

  #[test]
  fn test_different_error_strategies() {
    let mut transformer = TestTransformer::<i32>::new();

    // Test Stop strategy
    transformer = transformer
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop));
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));

    // Test Retry strategy
    transformer = transformer
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Retry
    ));
  }

  #[test]
  fn test_custom_error_handler() {
    let transformer = TestTransformer::<i32>::new().with_config(
      TransformerConfig::default()
        .with_error_strategy(ErrorStrategy::new_custom(|_| ErrorAction::Skip)),
    );

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));
  }

  #[test]
  fn test_component_info() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_name("test_transformer".to_string()));

    let info = transformer.component_info();
    assert_eq!(info.name, "test_transformer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<TestTransformer<i32>>()
    );
  }

  #[test]
  fn test_error_context_creation() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_name("test_transformer".to_string()));

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_transformer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<TestTransformer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_configuration_persistence() {
    let mut transformer = TestTransformer::<i32>::new().with_config(
      TransformerConfig::default()
        .with_name("test_transformer".to_string())
        .with_error_strategy(ErrorStrategy::Skip),
    );

    // Verify initial config
    assert_eq!(
      transformer.config().name(),
      Some("test_transformer".to_string())
    );
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));

    // Change config
    transformer = transformer.with_config(
      TransformerConfig::default()
        .with_name("new_name".to_string())
        .with_error_strategy(ErrorStrategy::Stop),
    );

    // Verify new config
    assert_eq!(transformer.config().name(), Some("new_name".to_string()));
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_multiple_configuration_changes() {
    let mut transformer = TestTransformer::<i32>::new();

    // First config change
    transformer = transformer.with_config(
      TransformerConfig::default()
        .with_name("first".to_string())
        .with_error_strategy(ErrorStrategy::Skip),
    );
    assert_eq!(transformer.config().name(), Some("first".to_string()));
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));

    // Second config change
    transformer = transformer.with_config(
      TransformerConfig::default()
        .with_name("second".to_string())
        .with_error_strategy(ErrorStrategy::Stop),
    );
    assert_eq!(transformer.config().name(), Some("second".to_string()));
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_thread_local_configuration() {
    let transformer1 = TestTransformer::<i32>::new().with_config(
      TransformerConfig::default()
        .with_name("transformer1".to_string())
        .with_error_strategy(ErrorStrategy::Skip),
    );

    let transformer2 = TestTransformer::<i32>::new().with_config(
      TransformerConfig::default()
        .with_name("transformer2".to_string())
        .with_error_strategy(ErrorStrategy::Stop),
    );

    assert_eq!(
      transformer1.config().name(),
      Some("transformer1".to_string())
    );
    assert!(matches!(
      transformer1.config().error_strategy(),
      ErrorStrategy::Skip
    ));
    assert_eq!(
      transformer2.config().name(),
      Some("transformer2".to_string())
    );
    assert!(matches!(
      transformer2.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_transformer_config_default() {
    let config = TransformerConfig::<i32>::default();
    assert!(matches!(config.error_strategy(), ErrorStrategy::Stop));
    assert_eq!(config.name(), None);
  }

  #[test]
  fn test_transformer_config_builder_pattern() {
    let config = TransformerConfig::<i32>::default()
      .with_name("test".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    assert_eq!(config.name(), Some("test".to_string()));
    assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
  }

  #[test]
  fn test_transformer_config_clone() {
    let config1 = TransformerConfig::<i32>::default()
      .with_name("test".to_string())
      .with_error_strategy(ErrorStrategy::Retry(3));

    let config2 = config1.clone();
    assert_eq!(config1, config2);
  }

  #[test]
  fn test_transformer_config_debug() {
    let config = TransformerConfig::<i32>::default()
      .with_name("test".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("test"));
  }

  #[test]
  fn test_transformer_config_partial_eq() {
    let config1 = TransformerConfig::<i32>::default()
      .with_name("test".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    let config2 = TransformerConfig::<i32>::default()
      .with_name("test".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    let config3 = TransformerConfig::<i32>::default()
      .with_name("different".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    assert_eq!(config1, config2);
    assert_ne!(config1, config3);
  }

  #[test]
  fn test_transformer_with_name() {
    let transformer = TestTransformer::<i32>::new().with_name("custom_name".to_string());

    assert_eq!(transformer.config().name(), Some("custom_name".to_string()));
  }

  #[test]
  fn test_transformer_with_config() {
    let config = TransformerConfig::<i32>::default()
      .with_name("test".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    let transformer = TestTransformer::<i32>::new().with_config(config.clone());
    assert_eq!(transformer.config(), &config);
  }

  #[test]
  fn test_transformer_config_mut() {
    let mut transformer = TestTransformer::<i32>::new();
    let config = transformer.config_mut();
    config.name = Some("test".to_string());

    assert_eq!(transformer.config().name(), Some("test".to_string()));
  }

  #[test]
  fn test_transformer_set_config() {
    let mut transformer = TestTransformer::<i32>::new();
    let config = TransformerConfig::<i32>::default()
      .with_name("test".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    transformer.set_config(config);
    assert_eq!(transformer.config().name(), Some("test".to_string()));
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_transformer_config_getter() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_name("test".to_string()));

    assert_eq!(transformer.config().name(), Some("test".to_string()));
  }

  #[test]
  fn test_error_handling_retry_exhausted() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(2)));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 3, // More than the retry limit
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_error_handling_retry_within_limit() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(5)));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 2, // Within the retry limit
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Retry
    ));
  }

  #[test]
  fn test_error_handling_stop_strategy() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_error_handling_skip_strategy() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));
  }

  #[test]
  fn test_component_info_default_name() {
    let transformer = TestTransformer::<i32>::new();
    let info = transformer.component_info();
    assert_eq!(info.name, "transformer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<TestTransformer<i32>>()
    );
  }

  #[test]
  fn test_error_context_with_none_item() {
    let transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_name("test".to_string()));

    let context = transformer.create_error_context(None);
    assert_eq!(context.component_name, "test");
    assert_eq!(
      context.component_type,
      std::any::type_name::<TestTransformer<i32>>()
    );
    assert_eq!(context.item, None);
  }

  #[test]
  fn test_error_context_timestamp() {
    let transformer = TestTransformer::<i32>::new();
    let before = chrono::Utc::now();
    let context = transformer.create_error_context(Some(42));
    let after = chrono::Utc::now();

    assert!(context.timestamp >= before);
    assert!(context.timestamp <= after);
  }

  #[tokio::test]
  async fn test_transformer_with_different_types() {
    let mut transformer = TestTransformer::<String>::new();
    let input = futures::stream::iter(vec!["hello".to_string(), "world".to_string()]);
    let output = transformer.transform(Box::pin(input));
    let result: Vec<String> = output.collect().await;
    assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
  }

  #[tokio::test]
  async fn test_transformer_with_large_input() {
    let mut transformer = TestTransformer::<i32>::new();
    let input = futures::stream::iter((1..=1000).collect::<Vec<_>>());
    let output = transformer.transform(Box::pin(input));
    let result: Vec<i32> = output.collect().await;
    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], 1);
    assert_eq!(result[999], 1000);
  }

  #[test]
  fn test_transformer_clone() {
    let transformer1 = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_name("test".to_string()));

    let transformer2 = transformer1.clone();
    assert_eq!(transformer1.config(), transformer2.config());
  }

  #[test]
  fn test_transformer_debug() {
    let transformer = TestTransformer::<i32>::new();
    let debug_str = format!("{:?}", transformer);
    assert!(debug_str.contains("TestTransformer"));
  }
}
