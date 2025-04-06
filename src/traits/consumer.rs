use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::input::Input;
use async_trait::async_trait;
use std::cell::RefCell;

thread_local! {
    static DEFAULT_CONFIG: RefCell<ConsumerConfig> = RefCell::new(ConsumerConfig {
        error_strategy: ErrorStrategy::Stop,
        name: String::new(),
    });
}

#[derive(Debug, Clone)]
pub struct ConsumerConfig {
  pub error_strategy: ErrorStrategy,
  pub name: String,
}

#[async_trait]
pub trait Consumer: Input {
  async fn consume(&mut self, stream: Self::InputStream) -> Result<(), Self::Error>;

  fn with_config(&self, config: ConsumerConfig) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  fn set_config(&mut self, config: ConsumerConfig) {
    DEFAULT_CONFIG.with(|c| *c.borrow_mut() = config);
  }

  fn get_config(&self) -> ConsumerConfig {
    DEFAULT_CONFIG.with(|c| c.borrow().clone())
  }

  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    let mut config = self.get_config();
    config.name = name;
    self.set_config(config);
    self
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.get_config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    let config = self.get_config();
    ComponentInfo {
      name: config.name,
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::{ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError};
  use crate::traits::error::Error;
  use futures::StreamExt;
  use std::error::Error as StdError;
  use std::fmt;
  use std::pin::Pin;
  use std::sync::{Arc, Mutex};
  use tokio_stream::Stream;

  // Test error type
  #[derive(Debug)]
  struct TestError(String);

  impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl StdError for TestError {}

  // Test consumer that collects items into a vector
  #[derive(Clone)]
  struct CollectorConsumer<T> {
    items: Arc<Mutex<Vec<T>>>,
    name: Option<String>,
  }

  impl<T> CollectorConsumer<T> {
    fn new() -> Self {
      Self {
        items: Arc::new(Mutex::new(Vec::new())),
        name: None,
      }
    }

    fn get_items(&self) -> Vec<T>
    where
      T: Clone,
    {
      self.items.lock().unwrap().clone()
    }
  }

  impl<T: Clone + Send + 'static> Error for CollectorConsumer<T> {
    type Error = TestError;
  }

  impl<T: Clone + Send + 'static> Input for CollectorConsumer<T> {
    type Input = T;
    type InputStream = Pin<Box<dyn Stream<Item = Result<T, TestError>> + Send>>;
  }

  #[async_trait]
  impl<T: Clone + Send + 'static> Consumer for CollectorConsumer<T> {
    async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), TestError> {
      while let Some(item) = stream.next().await {
        match item {
          Ok(value) => self.items.lock().unwrap().push(value),
          Err(e) => return Err(e),
        }
      }
      Ok(())
    }

    fn with_name(mut self, name: String) -> Self
    where
      Self: Sized,
    {
      self.name = Some(name);
      self
    }
  }

  // Test consumer that always fails
  #[derive(Clone)]
  struct FailingConsumer {
    name: Option<String>,
  }

  impl FailingConsumer {
    fn new() -> Self {
      Self { name: None }
    }
  }

  impl Error for FailingConsumer {
    type Error = TestError;
  }

  impl Input for FailingConsumer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  #[async_trait]
  impl Consumer for FailingConsumer {
    async fn consume(&mut self, _input: Self::InputStream) -> Result<(), TestError> {
      Err(TestError("always fails".to_string()))
    }

    fn with_name(mut self, name: String) -> Self
    where
      Self: Sized,
    {
      self.name = Some(name);
      self
    }
  }

  #[tokio::test]
  async fn test_collector_consumer() {
    let mut consumer = CollectorConsumer::new();
    let input = vec![1, 2, 3, 4, 5];
    let stream = Box::pin(tokio_stream::iter(input.clone()).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
    assert_eq!(consumer.get_items(), input);
  }

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let input: Vec<i32> = vec![];
    let stream = Box::pin(tokio_stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
    assert!(consumer.get_items().is_empty());
  }

  #[tokio::test]
  async fn test_string_consumer() {
    let mut consumer = CollectorConsumer::new();
    let input = vec!["hello".to_string(), "world".to_string()];
    let stream = Box::pin(tokio_stream::iter(input.clone()).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
    assert_eq!(consumer.get_items(), input);
  }

  #[tokio::test]
  async fn test_failing_consumer() {
    let mut consumer = FailingConsumer::new();
    let input = vec![1, 2, 3, 4, 5];
    let stream = Box::pin(tokio_stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_err());
    if let Err(TestError(msg)) = result {
      assert_eq!(msg, "always fails");
    } else {
      panic!("Expected TestError");
    }
  }

  #[tokio::test]
  async fn test_stream_error_propagation() {
    let mut consumer = CollectorConsumer::new();
    let input: Vec<Result<i32, TestError>> =
      vec![Ok(1), Ok(2), Err(TestError("test error".into())), Ok(4)];
    let stream = Box::pin(tokio_stream::iter(input));

    let result = consumer.consume(stream).await;
    assert!(result.is_err());
    assert_eq!(consumer.get_items(), vec![1, 2]);
  }

  #[test]
  fn test_consumer_error_handling_stop() {
    let consumer = CollectorConsumer::<i32>::new();
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Consumer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CollectorConsumer".to_string(),
      },
    };

    let action = consumer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Stop));
  }

  #[test]
  fn test_consumer_error_handling_skip() {
    let mut consumer = CollectorConsumer::<i32>::new();
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: "test".to_string(),
    });
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Consumer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CollectorConsumer".to_string(),
      },
    };

    let action = consumer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Skip));
  }

  #[test]
  fn test_consumer_error_handling_retry() {
    let mut consumer = CollectorConsumer::<i32>::new();
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: "test".to_string(),
    });
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Consumer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CollectorConsumer".to_string(),
      },
    };

    let action = consumer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Retry));
  }

  #[test]
  fn test_consumer_error_handling_retry_exhausted() {
    let mut consumer = CollectorConsumer::<i32>::new();
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: "test".to_string(),
    });
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Consumer,
      },
      retries: 3,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CollectorConsumer".to_string(),
      },
    };

    let action = consumer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Stop));
  }

  #[test]
  fn test_consumer_component_info() {
    let consumer = CollectorConsumer::<i32>::new().with_name("test_consumer".to_string());
    let info = consumer.component_info();
    assert_eq!(info.name, "test_consumer");
    assert_eq!(info.type_name, "CollectorConsumer");
  }

  #[test]
  fn test_consumer_create_error_context() {
    let consumer = CollectorConsumer::<i32>::new();
    let context = consumer.create_error_context(None);
    assert!(matches!(context.stage, PipelineStage::Consumer));
    assert!(context.item.is_none());
  }

  #[test]
  fn test_consumer_create_error_context_with_item() {
    let consumer = CollectorConsumer::<i32>::new();
    let item = Box::new(42) as Box<dyn std::any::Any + Send>;
    let context = consumer.create_error_context(Some(item));
    assert!(matches!(context.stage, PipelineStage::Consumer));
    assert!(context.item.is_some());
  }

  #[tokio::test]
  async fn test_multiple_configuration_changes() {
    let mut consumer = CollectorConsumer::<i32>::new();

    // First config
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: "first".to_string(),
    });
    assert_eq!(consumer.get_config().name, "first");
    assert!(matches!(
      consumer.get_config().error_strategy,
      ErrorStrategy::Skip
    ));

    // Second config
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: "second".to_string(),
    });
    assert_eq!(consumer.get_config().name, "second");
    assert!(matches!(
      consumer.get_config().error_strategy,
      ErrorStrategy::Retry(3)
    ));
  }

  #[tokio::test]
  async fn test_configuration_persistence() {
    let mut consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: "persistent".to_string(),
    });

    // First consume
    let input1 = vec![1, 2, 3];
    let stream1 = Box::pin(tokio_stream::iter(input1.clone()).map(Ok));
    consumer.consume(stream1).await.unwrap();

    // Second consume - config should persist
    let input2 = vec![4, 5, 6];
    let stream2 = Box::pin(tokio_stream::iter(input2.clone()).map(Ok));
    consumer.consume(stream2).await.unwrap();

    assert_eq!(consumer.get_config().name, "persistent");
    assert!(matches!(
      consumer.get_config().error_strategy,
      ErrorStrategy::Skip
    ));
    assert_eq!(consumer.get_items(), vec![1, 2, 3, 4, 5, 6]);
  }

  #[test]
  fn test_custom_error_handler() {
    let mut consumer = CollectorConsumer::<i32>::new();
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Custom(Box::new(|_| ErrorAction::Skip)),
      name: "custom".to_string(),
    });

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Consumer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CollectorConsumer".to_string(),
      },
    };

    let action = consumer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Skip));
  }

  #[derive(Debug)]
  struct DifferentError(String);

  impl fmt::Display for DifferentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Different error: {}", self.0)
    }
  }

  impl StdError for DifferentError {}

  #[test]
  fn test_different_error_types() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let error = StreamError {
      source: Box::new(DifferentError("different error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Consumer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CollectorConsumer".to_string(),
      },
    };

    let action = consumer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Stop));
  }

  #[tokio::test]
  async fn test_concurrent_consumption() {
    let consumer = Arc::new(tokio::sync::Mutex::new(CollectorConsumer::<i32>::new()));
    let mut handles = vec![];

    for i in 0..3 {
      let consumer = Arc::clone(&consumer);
      let handle = tokio::spawn(async move {
        let mut consumer = consumer.lock().await;
        let input = vec![i * 3 + 1, i * 3 + 2, i * 3 + 3];
        let stream = Box::pin(tokio_stream::iter(input).map(Ok));
        consumer.consume(stream).await.unwrap();
      });
      handles.push(handle);
    }

    for handle in handles {
      handle.await.unwrap();
    }

    let consumer = consumer.lock().await;
    let mut collected = consumer.get_items();
    collected.sort();
    assert_eq!(collected, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
  }

  #[tokio::test]
  async fn test_stream_cancellation() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let (tx, rx) = tokio::sync::oneshot::channel();

    let input = tokio_stream::wrappers::ReceiverStream::new(tokio::sync::mpsc::channel(1).1);
    let stream = Box::pin(input.map(Ok));

    let handle = tokio::spawn(async move {
      let result = consumer.consume(stream).await;
      tx.send(result).unwrap();
    });

    // Cancel the stream
    handle.abort();

    // Verify the result
    let result = rx.await.unwrap();
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_stream_backpressure() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok));

    let handle = tokio::spawn(async move {
      consumer.consume(stream).await.unwrap();
      consumer
    });

    // Send more items than the channel capacity
    for i in 0..10 {
      tx.send(i).await.unwrap();
    }

    let consumer = handle.await.unwrap();
    assert_eq!(consumer.get_items().len(), 10);
  }

  #[tokio::test]
  async fn test_stream_timeout() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok));

    let handle = tokio::spawn(async move {
      tokio::time::timeout(
        std::time::Duration::from_millis(100),
        consumer.consume(stream),
      )
      .await
    });

    // Don't send any items, should timeout
    let result = handle.await.unwrap();
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_thread_local_configuration() {
    let consumer1 = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: "consumer1".to_string(),
    });

    let consumer2 = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: "consumer2".to_string(),
    });

    assert_eq!(consumer1.get_config().name, "consumer1");
    assert_eq!(consumer2.get_config().name, "consumer2");
    assert!(matches!(
      consumer1.get_config().error_strategy,
      ErrorStrategy::Skip
    ));
    assert!(matches!(
      consumer2.get_config().error_strategy,
      ErrorStrategy::Retry(3)
    ));
  }
}
