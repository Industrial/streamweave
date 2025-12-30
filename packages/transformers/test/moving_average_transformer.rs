use futures::{StreamExt, stream};
use std::pin::Pin;
use streamweave_error::ErrorStrategy;
use streamweave_transformers::{MovingAverageState, MovingAverageTransformer};
use tokio_stream::Stream;

#[tokio::test]
async fn test_moving_average_basic() {
  let mut transformer = MovingAverageTransformer::new(3);
  let input = Box::pin(stream::iter(vec![1.0, 2.0, 3.0, 4.0, 5.0]));

  let output = transformer.transform(input).await;
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

  let output = transformer.transform(input).await;
  let results: Vec<f64> = output.collect().await;

  // With window size 1, output equals input
  assert_eq!(results, vec![5.0, 10.0, 15.0, 20.0]);
}

#[tokio::test]
async fn test_moving_average_large_window() {
  let mut transformer = MovingAverageTransformer::new(10);
  let input = Box::pin(stream::iter(vec![1.0, 2.0, 3.0]));

  let output = transformer.transform(input).await;
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

  let output = transformer.transform(input).await;
  let results: Vec<f64> = output.collect().await;

  assert!(results.is_empty());
}

#[tokio::test]
async fn test_moving_average_state_persistence() {
  let mut transformer = MovingAverageTransformer::new(3);

  // First batch
  let input1 = Box::pin(stream::iter(vec![3.0, 6.0, 9.0]));
  let output1 = transformer.transform(input1).await;
  let results1: Vec<f64> = output1.collect().await;
  assert_eq!(results1, vec![3.0, 4.5, 6.0]);

  // State should persist, window is [3, 6, 9]
  let input2 = Box::pin(stream::iter(vec![12.0]));
  let output2 = transformer.transform(input2).await;
  let results2: Vec<f64> = output2.collect().await;
  // Window becomes [6, 9, 12] -> avg 9.0
  assert_eq!(results2, vec![9.0]);
}

#[tokio::test]
async fn test_moving_average_state_reset() {
  let mut transformer = MovingAverageTransformer::new(3);

  // Process some items
  let input1 = Box::pin(stream::iter(vec![10.0, 20.0, 30.0]));
  let output1 = transformer.transform(input1).await;
  let _: Vec<f64> = output1.collect().await;

  // Reset state
  transformer.reset_state().unwrap();

  // Should start fresh
  let input2 = Box::pin(stream::iter(vec![1.0, 2.0]));
  let output2 = transformer.transform(input2).await;
  let results2: Vec<f64> = output2.collect().await;
  assert_eq!(results2, vec![1.0, 1.5]);
}

#[tokio::test]
async fn test_moving_average_get_state() {
  let mut transformer = MovingAverageTransformer::new(3);
  let input = Box::pin(stream::iter(vec![2.0, 4.0, 6.0, 8.0]));

  let output = transformer.transform(input).await;
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

  let output = transformer.transform(input).await;
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
fn test_moving_average_state_new() {
  let state = MovingAverageState::new(5);
  assert_eq!(state.window_size, 5);
  assert!(state.window.is_empty());
}

#[test]
fn test_moving_average_state_add_value() {
  let mut state = MovingAverageState::new(3);
  state.add_value(10.0);
  assert_eq!(state.window.len(), 1);
  assert_eq!(state.window[0], 10.0);
}

#[test]
fn test_moving_average_state_add_value_overflow() {
  let mut state = MovingAverageState::new(2);
  state.add_value(1.0);
  state.add_value(2.0);
  state.add_value(3.0); // Should remove 1.0
  assert_eq!(state.window.len(), 2);
  assert_eq!(state.window[0], 2.0);
  assert_eq!(state.window[1], 3.0);
}

#[test]
fn test_moving_average_state_average_empty() {
  let state = MovingAverageState::new(3);
  assert_eq!(state.average(), 0.0);
}

#[test]
fn test_moving_average_state_average_single() {
  let mut state = MovingAverageState::new(3);
  state.add_value(10.0);
  assert_eq!(state.average(), 10.0);
}

#[test]
fn test_moving_average_state_average_multiple() {
  let mut state = MovingAverageState::new(3);
  state.add_value(10.0);
  state.add_value(20.0);
  state.add_value(30.0);
  assert_eq!(state.average(), 20.0);
}

#[test]
fn test_moving_average_state_clone() {
  let mut state1 = MovingAverageState::new(3);
  state1.add_value(5.0);
  state1.add_value(10.0);

  let state2 = state1.clone();
  assert_eq!(state1.window, state2.window);
  assert_eq!(state1.window_size, state2.window_size);
  assert_eq!(state1.average(), state2.average());
}

#[test]
fn test_moving_average_transformer_new() {
  let transformer = MovingAverageTransformer::new(5);
  assert_eq!(transformer.window_size(), 5);
}

#[test]
#[should_panic(expected = "Window size must be greater than 0")]
fn test_moving_average_transformer_new_zero_panics() {
  let _ = MovingAverageTransformer::new(0);
}

#[test]
fn test_moving_average_transformer_with_name() {
  let transformer = MovingAverageTransformer::new(3).with_name("test_moving_avg".to_string());
  assert_eq!(
    transformer.config().name(),
    Some("test_moving_avg".to_string())
  );
}

#[test]
fn test_moving_average_transformer_with_error_strategy() {
  let transformer =
    MovingAverageTransformer::new(3).with_error_strategy(ErrorStrategy::<f64>::Skip);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_moving_average_transformer_clone() {
  let transformer1 = MovingAverageTransformer::new(5);
  let transformer2 = transformer1.clone();

  assert_eq!(transformer1.window_size(), transformer2.window_size());
  assert_eq!(transformer1.window_size, transformer2.window_size);
}

#[test]
fn test_moving_average_transformer_chaining() {
  let transformer = MovingAverageTransformer::new(7)
    .with_error_strategy(ErrorStrategy::<f64>::Retry(3))
    .with_name("chained_moving_avg".to_string());

  assert_eq!(transformer.window_size(), 7);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Retry(3)
  ));
  assert_eq!(
    transformer.config().name(),
    Some("chained_moving_avg".to_string())
  );
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
