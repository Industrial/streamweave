//! Transformer implementations for RunningSumTransformer.

use super::running_sum_transformer::RunningSumTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use std::fmt::Debug;
use std::ops::Add;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::Input;
use streamweave::Output;
use streamweave::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_stateful::{InMemoryStateStore, StateStore, StateStoreExt, StatefulTransformer};
use tokio_stream::Stream;

impl<T> Input for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

/// Wrapper struct to implement StateStore on `Arc<InMemoryStateStore<T>>`
#[derive(Debug)]
pub struct SharedStateStore<T: Clone + Send + Sync>(pub Arc<InMemoryStateStore<T>>);

impl<T: Clone + Send + Sync> Clone for SharedStateStore<T> {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

impl<T: Clone + Send + Sync> StateStore<T> for SharedStateStore<T> {
  fn get(&self) -> streamweave_stateful::StateResult<Option<T>> {
    self.0.get()
  }

  fn set(&self, state: T) -> streamweave_stateful::StateResult<()> {
    self.0.set(state)
  }

  fn update_with(
    &self,
    f: Box<dyn FnOnce(Option<T>) -> T + Send>,
  ) -> streamweave_stateful::StateResult<T> {
    self.0.update_with(f)
  }

  fn reset(&self) -> streamweave_stateful::StateResult<()> {
    self.0.reset()
  }

  fn is_initialized(&self) -> bool {
    self.0.is_initialized()
  }

  fn initial_state(&self) -> Option<T> {
    self.0.initial_state()
  }
}

#[async_trait]
impl<T> Transformer for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let state_store_clone = Arc::clone(&self.state_store);

    input
      .map(move |item| {
        state_store_clone
          .update(move |current_opt| {
            let current = current_opt.unwrap_or_default();
            current + item
          })
          .unwrap_or_else(|_| T::default())
      })
      .boxed()
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "running_sum_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl<T> StatefulTransformer for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type State = T;
  type Store = SharedStateStore<T>;

  fn state_store(&self) -> &Self::Store {
    // This is a bit awkward, but we need to return a reference to SharedStateStore
    // For now, we'll work around this by implementing the state methods directly
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state_store_mut(&mut self) -> &mut Self::Store {
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state(&self) -> streamweave_stateful::StateResult<Option<Self::State>> {
    self.state_store.get()
  }

  fn set_state(&self, state: Self::State) -> streamweave_stateful::StateResult<()> {
    self.state_store.set(state)
  }

  fn reset_state(&self) -> streamweave_stateful::StateResult<()> {
    self.state_store.reset()
  }

  fn has_state(&self) -> bool {
    self.state_store.is_initialized()
  }

  fn update_state<F>(&self, f: F) -> streamweave_stateful::StateResult<Self::State>
  where
    F: FnOnce(Option<Self::State>) -> Self::State + Send + 'static,
  {
    self.state_store.update_with(Box::new(f))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_running_sum_basic() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

    let output = transformer.transform(input);
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![1, 3, 6, 10, 15]);
  }

  #[tokio::test]
  async fn test_running_sum_with_initial_value() {
    let mut transformer = RunningSumTransformer::<i32>::with_initial(100);
    let input = Box::pin(stream::iter(vec![1, 2, 3]));

    let output = transformer.transform(input);
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![101, 103, 106]);
  }

  #[tokio::test]
  async fn test_running_sum_empty_input() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(vec![]));

    let output = transformer.transform(input);
    let results: Vec<i32> = output.collect().await;

    assert!(results.is_empty());
  }

  #[tokio::test]
  async fn test_running_sum_floats() {
    let mut transformer = RunningSumTransformer::<f64>::new();
    let input = Box::pin(stream::iter(vec![1.5, 2.5, 3.0]));

    let output = transformer.transform(input);
    let results: Vec<f64> = output.collect().await;

    assert_eq!(results, vec![1.5, 4.0, 7.0]);
  }

  #[tokio::test]
  async fn test_running_sum_state_persistence() {
    let mut transformer = RunningSumTransformer::<i32>::new();

    // First batch
    let input1 = Box::pin(stream::iter(vec![1, 2, 3]));
    let output1 = transformer.transform(input1);
    let results1: Vec<i32> = output1.collect().await;
    assert_eq!(results1, vec![1, 3, 6]);

    // State should persist, so second batch continues from 6
    let input2 = Box::pin(stream::iter(vec![4, 5]));
    let output2 = transformer.transform(input2);
    let results2: Vec<i32> = output2.collect().await;
    assert_eq!(results2, vec![10, 15]);
  }

  #[tokio::test]
  async fn test_running_sum_state_reset() {
    let mut transformer = RunningSumTransformer::<i32>::new();

    // Process some items
    let input1 = Box::pin(stream::iter(vec![1, 2, 3]));
    let output1 = transformer.transform(input1);
    let _: Vec<i32> = output1.collect().await;

    // Reset state
    transformer.reset_state().unwrap();

    // Should start from 0 again
    let input2 = Box::pin(stream::iter(vec![10, 20]));
    let output2 = transformer.transform(input2);
    let results2: Vec<i32> = output2.collect().await;
    assert_eq!(results2, vec![10, 30]);
  }

  #[tokio::test]
  async fn test_running_sum_get_state() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

    let output = transformer.transform(input);
    let _: Vec<i32> = output.collect().await;

    // Get final state
    let final_state = transformer.state().unwrap().unwrap();
    assert_eq!(final_state, 15);
  }

  #[tokio::test]
  async fn test_running_sum_component_info() {
    let transformer = RunningSumTransformer::<i32>::new().with_name("my_running_sum".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "my_running_sum");
    assert!(info.type_name.contains("RunningSumTransformer"));
  }

  #[tokio::test]
  async fn test_running_sum_negative_numbers() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input = Box::pin(stream::iter(vec![10, -3, 5, -7]));

    let output = transformer.transform(input);
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![10, 7, 12, 5]);
  }

  #[tokio::test]
  async fn test_running_sum_has_state() {
    let transformer = RunningSumTransformer::<i32>::new();
    assert!(transformer.has_state());
  }

  #[tokio::test]
  async fn test_running_sum_set_state() {
    let mut transformer = RunningSumTransformer::<i32>::new();

    // Set state directly
    transformer.set_state(100).unwrap();

    // Process some items
    let input = Box::pin(stream::iter(vec![1, 2, 3]));
    let output = transformer.transform(input);
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![101, 103, 106]);
  }
}
