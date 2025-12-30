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
  /// let output = transformer.transform(input).await;
  /// ```
  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream;

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
