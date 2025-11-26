use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::broadcast::broadcast_transformer::BroadcastTransformer;
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Transformer for BroadcastTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let num_consumers = self.broadcast_config.num_consumers;

    Box::pin(input.map(move |item| {
      // Clone the item for each consumer
      (0..num_consumers).map(|_| item.clone()).collect::<Vec<T>>()
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
        .unwrap_or_else(|| "broadcast_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_broadcast_basic() {
    let mut transformer = BroadcastTransformer::<i32>::new(3);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], vec![1, 1, 1]);
    assert_eq!(result[1], vec![2, 2, 2]);
    assert_eq!(result[2], vec![3, 3, 3]);
  }

  #[tokio::test]
  async fn test_broadcast_two_consumers() {
    let mut transformer = BroadcastTransformer::<i32>::new(2);
    let input = stream::iter(vec![10, 20, 30]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], vec![10, 10]);
    assert_eq!(result[1], vec![20, 20]);
    assert_eq!(result[2], vec![30, 30]);
  }

  #[tokio::test]
  async fn test_broadcast_single_consumer() {
    let mut transformer = BroadcastTransformer::<i32>::new(1);
    let input = stream::iter(vec![5, 10, 15]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], vec![5]);
    assert_eq!(result[1], vec![10]);
    assert_eq!(result[2], vec![15]);
  }

  #[tokio::test]
  async fn test_broadcast_empty_input() {
    let mut transformer = BroadcastTransformer::<i32>::new(3);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_broadcast_with_strings() {
    let mut transformer = BroadcastTransformer::<String>::new(2);
    let input = stream::iter(vec!["hello".to_string(), "world".to_string()]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<String>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], vec!["hello".to_string(), "hello".to_string()]);
    assert_eq!(result[1], vec!["world".to_string(), "world".to_string()]);
  }

  #[tokio::test]
  async fn test_broadcast_preserves_order() {
    let mut transformer = BroadcastTransformer::<i32>::new(4);
    let input = stream::iter(1..=10);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 10);
    for (i, copies) in result.iter().enumerate() {
      let expected_value = (i + 1) as i32;
      assert_eq!(copies.len(), 4);
      assert!(copies.iter().all(|&v| v == expected_value));
    }
  }

  #[tokio::test]
  async fn test_broadcast_large_fan_out() {
    let mut transformer = BroadcastTransformer::<i32>::new(100);
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 100);
    assert!(result[0].iter().all(|&v| v == 42));
  }

  #[test]
  fn test_broadcast_component_info() {
    let transformer = BroadcastTransformer::<i32>::new(2).with_name("my_broadcaster".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "my_broadcaster");
    assert!(info.type_name.contains("BroadcastTransformer"));
  }

  #[test]
  fn test_broadcast_default_component_info() {
    let transformer = BroadcastTransformer::<i32>::new(2);

    let info = transformer.component_info();
    assert_eq!(info.name, "broadcast_transformer");
  }

  #[test]
  fn test_broadcast_error_handling_stop() {
    let transformer = BroadcastTransformer::<i32>::new(2).with_error_strategy(ErrorStrategy::Stop);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "BroadcastTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "BroadcastTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_broadcast_error_handling_skip() {
    let transformer = BroadcastTransformer::<i32>::new(2).with_error_strategy(ErrorStrategy::Skip);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "BroadcastTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "BroadcastTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));
  }

  #[test]
  fn test_broadcast_error_handling_retry() {
    let transformer =
      BroadcastTransformer::<i32>::new(2).with_error_strategy(ErrorStrategy::Retry(3));

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "BroadcastTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "BroadcastTransformer".to_string(),
      },
      retries: 1,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Retry
    ));

    // Test retry exhausted
    let error_exhausted = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "BroadcastTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "BroadcastTransformer".to_string(),
      },
      retries: 3,
    };

    assert!(matches!(
      transformer.handle_error(&error_exhausted),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_broadcast_create_error_context() {
    let transformer = BroadcastTransformer::<i32>::new(2).with_name("test_bc".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_bc");
    assert_eq!(context.item, Some(42));
  }

  #[tokio::test]
  async fn test_broadcast_with_complex_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Event {
      id: u32,
      name: String,
    }

    let mut transformer = BroadcastTransformer::<Event>::new(3);
    let input = stream::iter(vec![
      Event {
        id: 1,
        name: "event1".to_string(),
      },
      Event {
        id: 2,
        name: "event2".to_string(),
      },
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<Event>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 3);
    assert!(result[0].iter().all(|e| e.id == 1 && e.name == "event1"));
    assert_eq!(result[1].len(), 3);
    assert!(result[1].iter().all(|e| e.id == 2 && e.name == "event2"));
  }

  #[tokio::test]
  async fn test_broadcast_reusability() {
    let mut transformer = BroadcastTransformer::<i32>::new(2);

    // First use
    let input1 = stream::iter(vec![1, 2]);
    let result1: Vec<Vec<i32>> = transformer.transform(Box::pin(input1)).collect().await;
    assert_eq!(result1, vec![vec![1, 1], vec![2, 2]]);

    // Second use
    let input2 = stream::iter(vec![3, 4]);
    let result2: Vec<Vec<i32>> = transformer.transform(Box::pin(input2)).collect().await;
    assert_eq!(result2, vec![vec![3, 3], vec![4, 4]]);
  }
}
