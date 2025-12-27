use crate::round_robin_transformer::RoundRobinTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use streamweave_core::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T> Transformer for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((usize, T),);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let num_consumers = self.rr_config.num_consumers;
    let current_index = Arc::clone(&self.current_index);

    Box::pin(input.map(move |item| {
      let index = if num_consumers == 0 {
        0
      } else {
        current_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % num_consumers
      };
      (index, item)
    }))
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
        .unwrap_or_else(|| "round_robin_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_round_robin_basic() {
    let mut transformer = RoundRobinTransformer::<i32>::new(3);
    let input = stream::iter(vec![10, 20, 30, 40, 50, 60]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 6);
    assert_eq!(result[0], (0, 10)); // First element to consumer 0
    assert_eq!(result[1], (1, 20)); // Second element to consumer 1
    assert_eq!(result[2], (2, 30)); // Third element to consumer 2
    assert_eq!(result[3], (0, 40)); // Fourth element wraps to consumer 0
    assert_eq!(result[4], (1, 50)); // Fifth element to consumer 1
    assert_eq!(result[5], (2, 60)); // Sixth element to consumer 2
  }

  #[tokio::test]
  async fn test_round_robin_two_consumers() {
    let mut transformer = RoundRobinTransformer::<i32>::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 5);
    assert_eq!(result[0], (0, 1));
    assert_eq!(result[1], (1, 2));
    assert_eq!(result[2], (0, 3));
    assert_eq!(result[3], (1, 4));
    assert_eq!(result[4], (0, 5));
  }

  #[tokio::test]
  async fn test_round_robin_single_consumer() {
    let mut transformer = RoundRobinTransformer::<i32>::new(1);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|(idx, _)| *idx == 0));
  }

  #[tokio::test]
  async fn test_round_robin_empty_input() {
    let mut transformer = RoundRobinTransformer::<i32>::new(3);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_round_robin_with_strings() {
    let mut transformer = RoundRobinTransformer::<String>::new(2);
    let input = stream::iter(vec![
      "a".to_string(),
      "b".to_string(),
      "c".to_string(),
      "d".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, String)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 4);
    assert_eq!(result[0], (0, "a".to_string()));
    assert_eq!(result[1], (1, "b".to_string()));
    assert_eq!(result[2], (0, "c".to_string()));
    assert_eq!(result[3], (1, "d".to_string()));
  }

  #[tokio::test]
  async fn test_round_robin_large_num_consumers() {
    let mut transformer = RoundRobinTransformer::<i32>::new(10);
    let input = stream::iter(1..=15);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 15);

    // Verify round-robin pattern
    for (i, (idx, _)) in result.iter().enumerate() {
      assert_eq!(*idx, i % 10);
    }
  }

  #[tokio::test]
  async fn test_round_robin_even_distribution() {
    let mut transformer = RoundRobinTransformer::<i32>::new(4);
    let input = stream::iter(1..=100);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).collect().await;

    // Count elements per consumer
    let mut counts = [0usize; 4];
    for (idx, _) in &result {
      counts[*idx] += 1;
    }

    // Distribution should be even (25 each)
    assert_eq!(counts[0], 25);
    assert_eq!(counts[1], 25);
    assert_eq!(counts[2], 25);
    assert_eq!(counts[3], 25);
  }

  #[test]
  fn test_round_robin_component_info() {
    let transformer = RoundRobinTransformer::<i32>::new(2).with_name("my_round_robin".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "my_round_robin");
    assert!(info.type_name.contains("RoundRobinTransformer"));
  }

  #[test]
  fn test_round_robin_default_component_info() {
    let transformer = RoundRobinTransformer::<i32>::new(2);

    let info = transformer.component_info();
    assert_eq!(info.name, "round_robin_transformer");
  }

  #[test]
  fn test_round_robin_error_handling_stop() {
    let transformer = RoundRobinTransformer::<i32>::new(2).with_error_strategy(ErrorStrategy::Stop);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "RoundRobinTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RoundRobinTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_round_robin_error_handling_skip() {
    let transformer = RoundRobinTransformer::<i32>::new(2).with_error_strategy(ErrorStrategy::Skip);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "RoundRobinTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RoundRobinTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));
  }

  #[test]
  fn test_round_robin_error_handling_retry() {
    let transformer =
      RoundRobinTransformer::<i32>::new(2).with_error_strategy(ErrorStrategy::Retry(3));

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "RoundRobinTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RoundRobinTransformer".to_string(),
      },
      retries: 1,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Retry
    ));
  }

  #[test]
  fn test_round_robin_create_error_context() {
    let transformer = RoundRobinTransformer::<i32>::new(2).with_name("test_rr".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_rr");
    assert_eq!(context.item, Some(42));
  }

  #[tokio::test]
  async fn test_round_robin_reusability() {
    let mut transformer = RoundRobinTransformer::<i32>::new(3);

    // First use
    let input1 = stream::iter(vec![1, 2, 3]);
    let result1: Vec<(usize, i32)> = transformer.transform(Box::pin(input1)).collect().await;
    assert_eq!(result1, vec![(0, 1), (1, 2), (2, 3)]);

    // Note: counter continues from previous use (atomic state is shared)
    let input2 = stream::iter(vec![4, 5, 6]);
    let result2: Vec<(usize, i32)> = transformer.transform(Box::pin(input2)).collect().await;
    assert_eq!(result2, vec![(0, 4), (1, 5), (2, 6)]);
  }

  #[tokio::test]
  async fn test_round_robin_preserves_order_within_consumer() {
    let mut transformer = RoundRobinTransformer::<i32>::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).collect().await;

    // Elements to consumer 0: 1, 3, 5, 7
    let consumer_0: Vec<i32> = result
      .iter()
      .filter(|(idx, _)| *idx == 0)
      .map(|(_, v)| *v)
      .collect();
    assert_eq!(consumer_0, vec![1, 3, 5, 7]);

    // Elements to consumer 1: 2, 4, 6, 8
    let consumer_1: Vec<i32> = result
      .iter()
      .filter(|(idx, _)| *idx == 1)
      .map(|(_, v)| *v)
      .collect();
    assert_eq!(consumer_1, vec![2, 4, 6, 8]);
  }

  #[tokio::test]
  async fn test_round_robin_with_complex_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Event {
      id: u32,
      name: String,
    }

    let mut transformer = RoundRobinTransformer::<Event>::new(2);
    let input = stream::iter(vec![
      Event {
        id: 1,
        name: "event1".to_string(),
      },
      Event {
        id: 2,
        name: "event2".to_string(),
      },
      Event {
        id: 3,
        name: "event3".to_string(),
      },
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, Event)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].0, 0);
    assert_eq!(result[0].1.id, 1);
    assert_eq!(result[1].0, 1);
    assert_eq!(result[1].1.id, 2);
    assert_eq!(result[2].0, 0);
    assert_eq!(result[2].1.id, 3);
  }
}
