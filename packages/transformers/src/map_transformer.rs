//! Map transformer for StreamWeave

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::borrow::Cow;
use std::pin::Pin;
use streamweave::{Input, Output, Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_graph::ZeroCopyTransformer;

/// A transformer that applies a function to each item in the stream.
///
/// This transformer takes each input item and applies a function to transform it
/// into an output item, creating a one-to-one mapping.
pub struct MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The function to apply to each input item.
  pub f: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<I>,
  /// Phantom data to track the input type parameter.
  pub _phantom_i: std::marker::PhantomData<I>,
  /// Phantom data to track the output type parameter.
  pub _phantom_o: std::marker::PhantomData<O>,
}

impl<F, I, O> MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MapTransformer` with the given function.
  ///
  /// # Arguments
  ///
  /// * `f` - The function to apply to each input item.
  pub fn new(f: F) -> Self {
    Self {
      f,
      config: TransformerConfig::default(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<I>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<F, I, O> Clone for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      config: self.config.clone(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }
}

impl<F, I, O> Input for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = I;
  type InputStream = Pin<Box<dyn Stream<Item = I> + Send>>;
}

impl<F, I, O> Output for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = O;
  type OutputStream = Pin<Box<dyn Stream<Item = O> + Send>>;
}

#[async_trait]
impl<F, I, O> Transformer for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (I,);
  type OutputPorts = (O,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let f = self.f.clone();
    Box::pin(input.map(f))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<I>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<I> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<I> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<I>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<I>) -> ErrorContext<I> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "map_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl<F, I, O> ZeroCopyTransformer for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Transforms an item using zero-copy semantics with `Cow`.
  ///
  /// For `MapTransformer`, this method handles both `Cow::Borrowed` and `Cow::Owned`
  /// inputs. Since mapping produces a new value, we always return `Cow::Owned`.
  ///
  /// # Zero-Copy Behavior
  ///
  /// - `Cow::Borrowed`: Clones the input before applying the transformation function
  /// - `Cow::Owned`: Uses the owned value directly (no additional clone)
  ///
  /// Both cases return `Cow::Owned` since the transformation produces a new value.
  fn transform_zero_copy<'a>(&mut self, input: Cow<'a, I>) -> Cow<'a, O> {
    // Extract the value from Cow (clone if Borrowed, move if Owned)
    let value = match input {
      Cow::Borrowed(borrowed) => borrowed.clone(), // Clone borrowed value
      Cow::Owned(owned) => owned,                 // Use owned value directly
    };

    // Apply the transformation function
    let transformed = (self.f)(value);

    // Always return Owned since transformation produces a new value
    Cow::Owned(transformed)
  }
}
