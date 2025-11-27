use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::input::Input;
use async_trait::async_trait;

/// Configuration for a consumer component.
///
/// This struct holds configuration options that control how a consumer
/// behaves, including error handling strategy and component naming.
#[derive(Debug, Clone)]
pub struct ConsumerConfig<T: std::fmt::Debug + Clone + Send + Sync> {
  /// The error handling strategy to use when processing items.
  pub error_strategy: ErrorStrategy<T>,
  /// The name of this consumer component.
  pub name: String,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Default for ConsumerConfig<T> {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: String::new(),
    }
  }
}

/// Trait for components that consume data streams.
///
/// Consumers are the end point of a pipeline. They receive processed items
/// and typically write them to a destination (file, database, console, etc.)
/// or perform some final action.
///
/// # Example
///
/// ```rust
/// use streamweave::prelude::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut consumer = VecConsumer::new();
/// let stream = futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
/// consumer.consume(stream).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Implementations
///
/// Common consumer implementations include:
/// - [`VecConsumer`] - Collects items into a vector
/// - [`FileConsumer`] - Writes data to files
/// - [`KafkaConsumer`] - Produces to Kafka topics
/// - [`ConsoleConsumer`] - Prints items to console
#[async_trait]
pub trait Consumer: Input
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// Consumes a stream of items.
  ///
  /// This method is called by the pipeline to process the final stream.
  /// The consumer should handle all items in the stream and perform the
  /// appropriate action (write to file, send to database, etc.).
  ///
  /// # Arguments
  ///
  /// * `stream` - The stream of items to consume
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::prelude::*;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut consumer = VecConsumer::new();
  /// let stream = futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
  /// consumer.consume(stream).await?;
  /// # Ok(())
  /// # }
  /// ```
  async fn consume(&mut self, stream: Self::InputStream);

  /// Creates a new consumer instance with the given configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The configuration to apply to the consumer.
  #[must_use]
  fn with_config(&self, config: ConsumerConfig<Self::Input>) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  /// Sets the configuration for this consumer.
  ///
  /// # Arguments
  ///
  /// * `config` - The configuration to set.
  fn set_config(&mut self, config: ConsumerConfig<Self::Input>) {
    self.set_config_impl(config);
  }

  /// Returns a reference to the consumer's configuration.
  fn config(&self) -> &ConsumerConfig<Self::Input> {
    self.get_config_impl()
  }

  /// Returns a mutable reference to the consumer's configuration.
  fn config_mut(&mut self) -> &mut ConsumerConfig<Self::Input> {
    self.get_config_mut_impl()
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  #[must_use]
  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    self.config_mut().name = name.clone();
    self
  }

  /// Handles an error according to the consumer's error strategy.
  ///
  /// # Arguments
  ///
  /// * `error` - The error that occurred.
  ///
  /// # Returns
  ///
  /// The action to take based on the error strategy.
  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match &self.config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  /// Returns information about this consumer component.
  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config().name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  /// Creates an error context for the given item.
  ///
  /// # Arguments
  ///
  /// * `item` - The item that caused the error, if any.
  ///
  /// # Returns
  ///
  /// An error context containing information about when and where the error occurred.
  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  /// Internal implementation for setting configuration.
  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>);
  /// Internal implementation for getting configuration.
  fn get_config_impl(&self) -> &ConsumerConfig<Self::Input>;
  /// Internal implementation for getting mutable configuration.
  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::{ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use futures::StreamExt;
  use std::pin::Pin;
  use std::sync::Arc;
  use tokio::sync::Mutex;
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

  // Test consumer that collects items into a vector
  #[derive(Clone)]
  struct CollectorConsumer<T: std::fmt::Debug + Clone + Send + Sync> {
    items: Arc<Mutex<Vec<T>>>,
    config: ConsumerConfig<T>,
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync> CollectorConsumer<T> {
    fn new() -> Self {
      Self {
        items: Arc::new(Mutex::new(Vec::new())),
        config: ConsumerConfig::default(),
      }
    }

    async fn get_items(&self) -> Vec<T>
    where
      T: Clone,
    {
      self.items.lock().await.clone()
    }
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for CollectorConsumer<T> {
    type Input = T;
    type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
  }

  #[async_trait]
  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Consumer for CollectorConsumer<T> {
    async fn consume(&mut self, mut stream: Self::InputStream) {
      while let Some(item) = stream.next().await {
        self.items.lock().await.push(item);
      }
    }

    fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> &ConsumerConfig<Self::Input> {
      &self.config
    }

    fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input> {
      &mut self.config
    }
  }

  // Test consumer that always fails
  #[derive(Clone)]
  struct FailingConsumer {
    config: ConsumerConfig<i32>,
  }

  impl FailingConsumer {
    fn new() -> Self {
      Self {
        config: ConsumerConfig::default(),
      }
    }
  }

  impl Input for FailingConsumer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
  }

  #[async_trait]
  impl Consumer for FailingConsumer {
    async fn consume(&mut self, _input: Self::InputStream) {
      // This consumer just drops the stream without processing
    }

    fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> &ConsumerConfig<Self::Input> {
      &self.config
    }

    fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input> {
      &mut self.config
    }
  }

  #[tokio::test]
  async fn test_collector_consumer() {
    let mut consumer = CollectorConsumer::new();
    let input = vec![1, 2, 3, 4, 5];
    let stream = Box::pin(tokio_stream::iter(input.clone()));
    consumer.consume(stream).await;
    assert_eq!(consumer.get_items().await, input);
  }

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let input: Vec<i32> = vec![];
    let stream = Box::pin(tokio_stream::iter(input));
    consumer.consume(stream).await;
    assert!(consumer.get_items().await.is_empty());
  }

  #[tokio::test]
  async fn test_string_consumer() {
    let mut consumer = CollectorConsumer::new();
    let input = vec!["hello".to_string(), "world".to_string()];
    let stream = Box::pin(tokio_stream::iter(input.clone()));
    consumer.consume(stream).await;
    assert_eq!(consumer.get_items().await, input);
  }

  #[tokio::test]
  async fn test_failing_consumer() {
    let mut consumer = FailingConsumer::new();
    let input = vec![1, 2, 3, 4, 5];
    let stream = Box::pin(tokio_stream::iter(input));
    consumer.consume(stream).await;
    // The failing consumer just drops the stream, so we can't verify anything
  }

  #[test]
  fn test_consumer_error_handling_stop() {
    let consumer = CollectorConsumer::<i32>::new();
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "CollectorConsumer".to_string(),
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
    let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: "test_consumer".to_string(),
    });
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
      retries: 0,
    };
    assert!(matches!(consumer.handle_error(&error), ErrorAction::Skip));
  }

  #[test]
  fn test_consumer_error_handling_retry() {
    let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: "test_consumer".to_string(),
    });
    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
      retries: 0,
    };
    assert!(matches!(consumer.handle_error(&error), ErrorAction::Retry));
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
        component_name: "test".to_string(),
        component_type: "CollectorConsumer".to_string(),
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
    let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Stop,
      name: "test_consumer".to_string(),
    });
    let info = consumer.component_info();
    assert_eq!(info.name, "test_consumer");
    assert_eq!(
      info.type_name,
      "streamweave::consumer::tests::CollectorConsumer<i32>"
    );
  }

  #[test]
  fn test_consumer_create_error_context() {
    let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Stop,
      name: "test_consumer".to_string(),
    });
    let context = consumer.create_error_context(None);
    assert_eq!(context.component_name, "test_consumer");
    assert_eq!(
      context.component_type,
      "streamweave::consumer::tests::CollectorConsumer<i32>"
    );
    assert!(context.item.is_none());
  }

  #[test]
  fn test_consumer_create_error_context_with_item() {
    let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Stop,
      name: "test_consumer".to_string(),
    });
    let context = consumer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_consumer");
    assert_eq!(
      context.component_type,
      "streamweave::consumer::tests::CollectorConsumer<i32>"
    );
    assert_eq!(context.item, Some(42));
  }

  #[tokio::test]
  async fn test_multiple_configuration_changes() {
    let mut consumer = CollectorConsumer::<i32>::new();

    // First config
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: "first".to_string(),
    });
    assert_eq!(consumer.config().name, "first");
    assert!(matches!(
      consumer.config().error_strategy,
      ErrorStrategy::Skip
    ));

    // Second config
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: "second".to_string(),
    });
    assert_eq!(consumer.config().name, "second");
    assert!(matches!(
      consumer.config().error_strategy,
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
    let stream1 = Box::pin(tokio_stream::iter(input1.clone()));
    consumer.consume(stream1).await;

    // Second consume - config should persist
    let input2 = vec![4, 5, 6];
    let stream2 = Box::pin(tokio_stream::iter(input2.clone()));
    consumer.consume(stream2).await;

    assert_eq!(consumer.config().name, "persistent");
    assert!(matches!(
      consumer.config().error_strategy,
      ErrorStrategy::Skip
    ));
    assert_eq!(consumer.get_items().await, vec![1, 2, 3, 4, 5, 6]);
  }

  #[test]
  fn test_custom_error_handler() {
    let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::new_custom(|_| ErrorAction::Skip),
      name: "test_consumer".to_string(),
    });

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    };

    let action = consumer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Skip));
  }

  #[derive(Debug)]
  struct DifferentError(String);
  impl std::fmt::Display for DifferentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }
  impl std::error::Error for DifferentError {}

  #[test]
  fn test_different_error_types() {
    let consumer = CollectorConsumer::<i32>::new();
    let error = StreamError {
      source: Box::new(DifferentError("different error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "CollectorConsumer".to_string(),
        component_type: "CollectorConsumer".to_string(),
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
        let stream = Box::pin(tokio_stream::iter(input));
        consumer.consume(stream).await;
      });
      handles.push(handle);
    }
    for handle in handles {
      handle.await.unwrap();
    }
    let consumer = consumer.lock().await;
    let mut collected = consumer.get_items().await;
    collected.sort();
    assert_eq!(collected, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
  }

  #[tokio::test]
  async fn test_stream_cancellation() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let input = tokio_stream::wrappers::ReceiverStream::new(tokio::sync::mpsc::channel(1).1);
    let stream = Box::pin(input);
    let handle = tokio::spawn(async move {
      consumer.consume(stream).await;
      tx.send(()).unwrap();
    });
    // Cancel the stream
    handle.abort();
    // Verify the result
    let _ = rx.await;
  }

  #[tokio::test]
  async fn test_stream_backpressure() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));
    let handle = tokio::spawn(async move {
      consumer.consume(stream).await;
      consumer
    });
    // Send more items than the channel capacity
    for i in 0..10 {
      tx.send(i).await.unwrap();
    }
    // Close the sender to signal stream completion
    drop(tx);
    let consumer = handle.await.unwrap();
    assert_eq!(consumer.get_items().await.len(), 10);
  }

  #[tokio::test]
  async fn test_stream_timeout() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let (_tx, rx) = tokio::sync::mpsc::channel(1);
    let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));
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
    assert_eq!(consumer1.config().name, "consumer1");
    assert_eq!(consumer2.config().name, "consumer2");
    assert!(matches!(
      consumer1.config().error_strategy,
      ErrorStrategy::Skip
    ));
    assert!(matches!(
      consumer2.config().error_strategy,
      ErrorStrategy::Retry(3)
    ));
  }
}
