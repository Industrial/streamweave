//! Transformer implementations for MovingAverageTransformer.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::input::Input;
use crate::output::Output;
use crate::stateful_transformer::{
  InMemoryStateStore, StateStore, StateStoreExt, StatefulTransformer,
};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::moving_average::moving_average_transformer::{
  MovingAverageState, MovingAverageTransformer,
};
use async_trait::async_trait;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

impl Input for MovingAverageTransformer {
  type Input = f64;
  type InputStream = Pin<Box<dyn Stream<Item = f64> + Send>>;
}

impl Output for MovingAverageTransformer {
  type Output = f64;
  type OutputStream = Pin<Box<dyn Stream<Item = f64> + Send>>;
}

/// Wrapper struct to implement StateStore on Arc<InMemoryStateStore<MovingAverageState>>
#[derive(Debug)]
pub struct SharedMovingAverageStore(pub Arc<InMemoryStateStore<MovingAverageState>>);

impl Clone for SharedMovingAverageStore {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

impl StateStore<MovingAverageState> for SharedMovingAverageStore {
  fn get(&self) -> crate::stateful_transformer::StateResult<Option<MovingAverageState>> {
    self.0.get()
  }

  fn set(&self, state: MovingAverageState) -> crate::stateful_transformer::StateResult<()> {
    self.0.set(state)
  }

  fn update_with(
    &self,
    f: Box<dyn FnOnce(Option<MovingAverageState>) -> MovingAverageState + Send>,
  ) -> crate::stateful_transformer::StateResult<MovingAverageState> {
    self.0.update_with(f)
  }

  fn reset(&self) -> crate::stateful_transformer::StateResult<()> {
    self.0.reset()
  }

  fn is_initialized(&self) -> bool {
    self.0.is_initialized()
  }

  fn initial_state(&self) -> Option<MovingAverageState> {
    self.0.initial_state()
  }
}

#[async_trait]
impl Transformer for MovingAverageTransformer {
  type InputPorts = (f64,);
  type OutputPorts = (f64,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let state_store_clone = Arc::clone(&self.state_store);
    let window_size = self.window_size;

    input
      .map(move |item| {
        state_store_clone
          .update(move |current_opt| {
            let mut state = current_opt.unwrap_or_else(|| MovingAverageState::new(window_size));
            state.add_value(item);
            state
          })
          .map(|state| state.average())
          .unwrap_or(0.0)
      })
      .boxed()
  }

  fn set_config_impl(&mut self, config: TransformerConfig<f64>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<f64> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<f64> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<f64>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<f64>) -> ErrorContext<f64> {
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
        .unwrap_or_else(|| "moving_average_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl StatefulTransformer for MovingAverageTransformer {
  type State = MovingAverageState;
  type Store = SharedMovingAverageStore;

  fn state_store(&self) -> &Self::Store {
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state_store_mut(&mut self) -> &mut Self::Store {
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state(&self) -> crate::stateful_transformer::StateResult<Option<Self::State>> {
    self.state_store.get()
  }

  fn set_state(&self, state: Self::State) -> crate::stateful_transformer::StateResult<()> {
    self.state_store.set(state)
  }

  fn reset_state(&self) -> crate::stateful_transformer::StateResult<()> {
    self.state_store.reset()
  }

  fn has_state(&self) -> bool {
    self.state_store.is_initialized()
  }

  fn update_state<F>(&self, f: F) -> crate::stateful_transformer::StateResult<Self::State>
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
  async fn test_moving_average_basic() {
    let mut transformer = MovingAverageTransformer::new(3);
    let input = Box::pin(stream::iter(vec![1.0, 2.0, 3.0, 4.0, 5.0]));

    let output = transformer.transform(input);
    let results: Vec<f64> = output.collect().await;

    // Window progression:
    // [1] -> 1.0
    // [1,2] -> 1.5
    // [1,2,3] -> 2.0
    // [2,3,4] -> 3.0
    // [3,4,5] -> 4.0
    assert_eq!(results, vec![1.0, 1.5, 2.0, 3.0, 4.0]);
  }

  #[tokio::test]
  async fn test_moving_average_window_size_1() {
    let mut transformer = MovingAverageTransformer::new(1);
    let input = Box::pin(stream::iter(vec![5.0, 10.0, 15.0, 20.0]));

    let output = transformer.transform(input);
    let results: Vec<f64> = output.collect().await;

    // With window size 1, output equals input
    assert_eq!(results, vec![5.0, 10.0, 15.0, 20.0]);
  }

  #[tokio::test]
  async fn test_moving_average_large_window() {
    let mut transformer = MovingAverageTransformer::new(10);
    let input = Box::pin(stream::iter(vec![1.0, 2.0, 3.0]));

    let output = transformer.transform(input);
    let results: Vec<f64> = output.collect().await;

    // Window never fills, so we get cumulative averages
    // [1] -> 1.0
    // [1,2] -> 1.5
    // [1,2,3] -> 2.0
    assert_eq!(results, vec![1.0, 1.5, 2.0]);
  }

  #[tokio::test]
  async fn test_moving_average_empty_input() {
    let mut transformer = MovingAverageTransformer::new(3);
    let input: Pin<Box<dyn Stream<Item = f64> + Send>> = Box::pin(stream::iter(vec![]));

    let output = transformer.transform(input);
    let results: Vec<f64> = output.collect().await;

    assert!(results.is_empty());
  }

  #[tokio::test]
  async fn test_moving_average_state_persistence() {
    let mut transformer = MovingAverageTransformer::new(3);

    // First batch
    let input1 = Box::pin(stream::iter(vec![3.0, 6.0, 9.0]));
    let output1 = transformer.transform(input1);
    let results1: Vec<f64> = output1.collect().await;
    assert_eq!(results1, vec![3.0, 4.5, 6.0]);

    // State should persist, window is [3, 6, 9]
    let input2 = Box::pin(stream::iter(vec![12.0]));
    let output2 = transformer.transform(input2);
    let results2: Vec<f64> = output2.collect().await;
    // Window becomes [6, 9, 12] -> avg 9.0
    assert_eq!(results2, vec![9.0]);
  }

  #[tokio::test]
  async fn test_moving_average_state_reset() {
    let mut transformer = MovingAverageTransformer::new(3);

    // Process some items
    let input1 = Box::pin(stream::iter(vec![10.0, 20.0, 30.0]));
    let output1 = transformer.transform(input1);
    let _: Vec<f64> = output1.collect().await;

    // Reset state
    transformer.reset_state().unwrap();

    // Should start fresh
    let input2 = Box::pin(stream::iter(vec![1.0, 2.0]));
    let output2 = transformer.transform(input2);
    let results2: Vec<f64> = output2.collect().await;
    assert_eq!(results2, vec![1.0, 1.5]);
  }

  #[tokio::test]
  async fn test_moving_average_get_state() {
    let mut transformer = MovingAverageTransformer::new(3);
    let input = Box::pin(stream::iter(vec![2.0, 4.0, 6.0, 8.0]));

    let output = transformer.transform(input);
    let _: Vec<f64> = output.collect().await;

    // Get final state
    let final_state = transformer.state().unwrap().unwrap();
    // Window should be [4, 6, 8]
    assert_eq!(final_state.window.len(), 3);
    assert_eq!(final_state.average(), 6.0);
  }

  #[tokio::test]
  async fn test_moving_average_component_info() {
    let transformer = MovingAverageTransformer::new(5).with_name("my_moving_avg".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "my_moving_avg");
    assert!(info.type_name.contains("MovingAverageTransformer"));
  }

  #[tokio::test]
  async fn test_moving_average_window_size() {
    let transformer = MovingAverageTransformer::new(7);
    assert_eq!(transformer.window_size(), 7);
  }

  #[tokio::test]
  async fn test_moving_average_negative_values() {
    let mut transformer = MovingAverageTransformer::new(2);
    let input = Box::pin(stream::iter(vec![10.0, -4.0, 6.0]));

    let output = transformer.transform(input);
    let results: Vec<f64> = output.collect().await;

    // [10] -> 10.0
    // [10, -4] -> 3.0
    // [-4, 6] -> 1.0
    assert_eq!(results, vec![10.0, 3.0, 1.0]);
  }

  #[tokio::test]
  async fn test_moving_average_has_state() {
    let transformer = MovingAverageTransformer::new(3);
    assert!(transformer.has_state());
  }

  #[tokio::test]
  #[should_panic(expected = "Window size must be greater than 0")]
  async fn test_moving_average_zero_window_panics() {
    let _ = MovingAverageTransformer::new(0);
  }

  #[test]
  fn test_moving_average_state_struct() {
    let mut state = MovingAverageState::new(3);

    assert_eq!(state.average(), 0.0); // Empty window

    state.add_value(6.0);
    assert_eq!(state.average(), 6.0);

    state.add_value(12.0);
    assert_eq!(state.average(), 9.0);

    state.add_value(9.0);
    assert_eq!(state.average(), 9.0);

    state.add_value(3.0);
    assert_eq!(state.average(), 8.0); // [12, 9, 3]
  }
}
