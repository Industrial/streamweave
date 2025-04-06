use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::output::Output;
use std::cell::RefCell;

thread_local! {
    static DEFAULT_CONFIG: RefCell<ProducerConfig> = RefCell::new(ProducerConfig::default());
}

pub struct ProducerConfig {
  error_strategy: ErrorStrategy,
  name: Option<String>,
}

impl Default for ProducerConfig {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: None,
    }
  }
}

pub trait Producer: Output {
  fn produce(&mut self) -> Self::OutputStream;

  fn config(&self) -> &ProducerConfig {
    DEFAULT_CONFIG.with(|config| unsafe { &*config.as_ptr() })
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
    DEFAULT_CONFIG.with(|config| unsafe { &mut *config.as_ptr() })
  }

  fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self
  where
    Self: Sized,
  {
    self.config_mut().error_strategy = strategy;
    self
  }

  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    self.config_mut().name = Some(name);
    self
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match &self.config().error_strategy {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(max_retries) => {
        if error.retries < *max_retries {
          ErrorStrategy::Retry(*max_retries)
        } else {
          ErrorStrategy::Stop
        }
      }
      ErrorStrategy::Custom(handler) => {
        match handler(&error) {
          ErrorAction::Stop => ErrorStrategy::Stop,
          ErrorAction::Skip => ErrorStrategy::Skip,
          ErrorAction::Retry => ErrorStrategy::Retry(3), // Default retry count
        }
      }
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name
        .clone()
        .unwrap_or_else(|| "producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
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
  #[derive(Debug, Clone)]
  struct TestError(String);

  impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl StdError for TestError {}

  // Test producer implementation
  struct TestProducer {
    data: Vec<Result<i32, TestError>>,
    config: ProducerConfig,
  }

  impl TestProducer {
    fn new(data: Vec<Result<i32, TestError>>) -> Self {
      Self {
        data,
        config: ProducerConfig::default(),
      }
    }

    fn with_config(mut self, config: ProducerConfig) -> Self {
      self.config = config;
      self
    }
  }

  impl Error for TestProducer {
    type Error = TestError;
  }

  impl Output for TestProducer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Producer for TestProducer {
    fn produce(&mut self) -> Self::OutputStream {
      Box::pin(tokio_stream::iter(self.data.clone()))
    }

    fn with_name(mut self, name: String) -> Self
    where
      Self: Sized,
    {
      self.config.name = Some(name);
      self
    }

    fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self
    where
      Self: Sized,
    {
      self.config.error_strategy = strategy;
      self
    }

    fn config(&self) -> &ProducerConfig {
      &self.config
    }

    fn config_mut(&mut self) -> &mut ProducerConfig {
      &mut self.config
    }

    fn handle_error(&self, error: StreamError) -> ErrorStrategy {
      match &self.config.error_strategy {
        ErrorStrategy::Stop => ErrorStrategy::Stop,
        ErrorStrategy::Skip => ErrorStrategy::Skip,
        ErrorStrategy::Retry(max_retries) => {
          if error.retries < *max_retries {
            ErrorStrategy::Retry(*max_retries)
          } else {
            ErrorStrategy::Stop
          }
        }
        ErrorStrategy::Custom(handler) => {
          match handler(&error) {
            ErrorAction::Stop => ErrorStrategy::Stop,
            ErrorAction::Skip => ErrorStrategy::Skip,
            ErrorAction::Retry => ErrorStrategy::Retry(3), // Default retry count
          }
        }
      }
    }
  }

  #[tokio::test]
  async fn test_producer_successful_stream() {
    let mut producer = TestProducer::new(vec![Ok(1), Ok(2), Ok(3)]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_producer_error_stream() {
    let mut producer =
      TestProducer::new(vec![Ok(1), Err(TestError("test error".to_string())), Ok(3)]);
    let stream = producer.produce();
    let result: Vec<Result<i32, TestError>> = stream.collect().await;
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].as_ref().unwrap(), &1);
    assert!(result[1].is_err());
    assert_eq!(result[2].as_ref().unwrap(), &3);
  }

  #[tokio::test]
  async fn test_producer_empty_stream() {
    let mut producer = TestProducer::new(vec![]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert!(result.is_empty());
  }

  #[test]
  fn test_producer_error_handling_stop() {
    let producer = TestProducer::new(vec![]);
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Stop));
  }

  #[test]
  fn test_producer_error_handling_skip() {
    let mut producer = TestProducer::new(vec![]);
    producer = producer.with_error_strategy(ErrorStrategy::Skip);
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Skip));
  }

  #[test]
  fn test_producer_error_handling_retry() {
    let mut producer = TestProducer::new(vec![]);
    producer = producer.with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Retry(3)));
  }

  #[test]
  fn test_producer_error_handling_retry_exhausted() {
    let mut producer = TestProducer::new(vec![]);
    producer = producer.with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 3,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Stop));
  }

  #[test]
  fn test_producer_error_handling_custom() {
    let mut producer = TestProducer::new(vec![]);
    producer = producer.with_error_strategy(ErrorStrategy::Custom(Box::new(|_| ErrorAction::Skip)));
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Skip));
  }

  #[test]
  fn test_producer_component_info() {
    let producer = TestProducer::new(vec![]).with_name("test_producer".to_string());
    let info = producer.component_info();
    assert_eq!(info.name, "test_producer");
    assert_eq!(info.type_name, "TestProducer");
  }

  #[test]
  fn test_producer_create_error_context() {
    let producer = TestProducer::new(vec![]);
    let context = producer.create_error_context(None);
    assert!(matches!(context.stage, PipelineStage::Producer));
    assert!(context.item.is_none());
  }

  #[test]
  fn test_producer_create_error_context_with_item() {
    let producer = TestProducer::new(vec![]);
    let item = Box::new(42) as Box<dyn std::any::Any + Send>;
    let context = producer.create_error_context(Some(item));
    assert!(matches!(context.stage, PipelineStage::Producer));
    assert!(context.item.is_some());
  }

  #[tokio::test]
  async fn test_multiple_configuration_changes() {
    let mut producer = TestProducer::new(vec![]);

    // First config
    producer = producer.with_config(ProducerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: Some("first".to_string()),
    });
    assert_eq!(producer.config().name, Some("first".to_string()));
    assert!(matches!(
      producer.config().error_strategy,
      ErrorStrategy::Skip
    ));

    // Second config
    producer = producer.with_config(ProducerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: Some("second".to_string()),
    });
    assert_eq!(producer.config().name, Some("second".to_string()));
    assert!(matches!(
      producer.config().error_strategy,
      ErrorStrategy::Retry(3)
    ));
  }

  #[tokio::test]
  async fn test_configuration_persistence() {
    let mut producer = TestProducer::new(vec![]).with_config(ProducerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: Some("persistent".to_string()),
    });

    // First produce
    let stream1 = producer.produce();
    let result1: Vec<i32> = stream1.map(|r| r.unwrap()).collect().await;

    // Second produce - config should persist
    let stream2 = producer.produce();
    let result2: Vec<i32> = stream2.map(|r| r.unwrap()).collect().await;

    assert_eq!(producer.config().name, Some("persistent".to_string()));
    assert!(matches!(
      producer.config().error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[tokio::test]
  async fn test_concurrent_production() {
    let producer = Arc::new(tokio::sync::Mutex::new(TestProducer::new(vec![
      Ok(1),
      Ok(2),
      Ok(3),
    ])));
    let mut handles = vec![];

    for _ in 0..3 {
      let producer = Arc::clone(&producer);
      let handle = tokio::spawn(async move {
        let mut producer = producer.lock().await;
        let stream = producer.produce();
        let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
        result
      });
      handles.push(handle);
    }

    for handle in handles {
      let result = handle.await.unwrap();
      assert_eq!(result, vec![1, 2, 3]);
    }
  }

  #[tokio::test]
  async fn test_stream_cancellation() {
    let mut producer = TestProducer::new(vec![Ok(1), Ok(2), Ok(3)]);
    let (tx, rx) = tokio::sync::oneshot::channel();

    let stream = producer.produce();
    let handle = tokio::spawn(async move {
      let result: Vec<Result<i32, TestError>> = stream.collect().await;
      tx.send(result).unwrap();
    });

    // Cancel the stream
    handle.abort();

    // Verify the result
    let result = rx.await.unwrap();
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_stream_backpressure() {
    let mut producer = TestProducer::new(vec![Ok(1), Ok(2), Ok(3)]);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let stream = producer.produce();

    let handle = tokio::spawn(async move {
      let mut stream = stream;
      while let Some(item) = stream.next().await {
        tx.send(item).await.unwrap();
      }
    });

    // Consume items with delay to create backpressure
    for _ in 0..3 {
      let item = rx.recv().await.unwrap();
      assert!(item.is_ok());
      tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    handle.await.unwrap();
  }

  #[tokio::test]
  async fn test_stream_timeout() {
    let mut producer = TestProducer::new(vec![Ok(1), Ok(2), Ok(3)]);
    let stream = producer.produce();

    let handle = tokio::spawn(async move {
      tokio::time::timeout(
        std::time::Duration::from_millis(100),
        stream.collect::<Vec<_>>(),
      )
      .await
    });

    let result = handle.await.unwrap();
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_thread_local_configuration() {
    let producer1 = TestProducer::new(vec![]).with_config(ProducerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: Some("producer1".to_string()),
    });

    let producer2 = TestProducer::new(vec![]).with_config(ProducerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: Some("producer2".to_string()),
    });

    assert_eq!(producer1.config().name, Some("producer1".to_string()));
    assert_eq!(producer2.config().name, Some("producer2".to_string()));
    assert!(matches!(
      producer1.config().error_strategy,
      ErrorStrategy::Skip
    ));
    assert!(matches!(
      producer2.config().error_strategy,
      ErrorStrategy::Retry(3)
    ));
  }

  #[tokio::test]
  async fn test_producer_with_different_error_types() {
    #[derive(Debug)]
    struct DifferentError(String);

    impl fmt::Display for DifferentError {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Different error: {}", self.0)
      }
    }

    impl StdError for DifferentError {}

    let mut producer = TestProducer::new(vec![]);
    let error = StreamError {
      source: Box::new(DifferentError("different error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Stop));
  }
}
