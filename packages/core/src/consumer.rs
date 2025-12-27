use crate::input::Input;
use crate::port::PortList;
use async_trait::async_trait;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

/// Helper trait for providing default port types for Consumers.
///
/// This trait provides default implementations of the `InputPorts` associated type.
/// All Consumers automatically get `InputPorts = (Self::Input,)` unless they
/// explicitly override it.
pub trait ConsumerPorts: Consumer
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The default input port tuple type (single port with the consumer's input type).
  type DefaultInputPorts: PortList;
}

/// Blanket implementation: all Consumers get default single-port input.
impl<C> ConsumerPorts for C
where
  C: Consumer,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
{
  type DefaultInputPorts = (C::Input,);
}

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
/// - `VecConsumer` - Collects items into a vector
/// - `FileConsumer` - Writes data to files
/// - `KafkaConsumer` - Produces to Kafka topics
/// - `ConsoleConsumer` - Prints items to console
#[async_trait]
pub trait Consumer: Input
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The input port tuple type for this consumer.
  ///
  /// This associated type specifies the port tuple that represents this consumer's
  /// inputs in the graph API. By default, consumers have a single input port
  /// containing their input type: `(Self::Input,)`.
  ///
  /// For multi-port consumers, override this type to specify a tuple with multiple
  /// input types, e.g., `(i32, String)` for two inputs.
  type InputPorts: PortList;

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
  use futures::StreamExt;
  use proptest::prelude::*;
  use std::pin::Pin;
  use std::sync::Arc;
  use streamweave_error::{ErrorAction, ErrorContext, ErrorStrategy, StreamError};
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
    type InputPorts = (T,);

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
    type InputPorts = (i32,);

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

  // Helper function for async tests with proptest
  async fn test_consumer_with_input(input: Vec<i32>) {
    let mut consumer = CollectorConsumer::new();
    let stream = Box::pin(tokio_stream::iter(input.clone()));
    consumer.consume(stream).await;
    assert_eq!(consumer.get_items().await, input);
  }

  #[test]
  fn test_collector_consumer() {
    proptest::proptest!(|(input in prop::collection::vec(-1000..1000i32, 0..100))| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_consumer_with_input(input));
    });
  }

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let input: Vec<i32> = vec![];
    let stream = Box::pin(tokio_stream::iter(input));
    consumer.consume(stream).await;
    assert!(consumer.get_items().await.is_empty());
  }

  async fn test_string_consumer_with_input(input: Vec<String>) {
    let mut consumer = CollectorConsumer::new();
    let stream = Box::pin(tokio_stream::iter(input.clone()));
    consumer.consume(stream).await;
    assert_eq!(consumer.get_items().await, input);
  }

  #[test]
  fn test_string_consumer() {
    proptest::proptest!(|(input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(), 0..50))| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_string_consumer_with_input(input));
    });
  }

  async fn test_failing_consumer_with_input(input: Vec<i32>) {
    let mut consumer = FailingConsumer::new();
    let stream = Box::pin(tokio_stream::iter(input));
    consumer.consume(stream).await;
    // The failing consumer just drops the stream, so we can't verify anything
  }

  #[test]
  fn test_failing_consumer() {
    proptest::proptest!(|(input in prop::collection::vec(-1000..1000i32, 0..100))| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_failing_consumer_with_input(input));
    });
  }

  proptest! {
    #[test]
    fn test_consumer_error_handling_stop(
      error_msg in prop::string::string_regex(".+").unwrap(),
      retries in 0..10usize
    ) {
      let consumer = CollectorConsumer::<i32>::new();
      let error = StreamError {
        source: Box::new(TestError(error_msg.clone())),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "test".to_string(),
          component_type: "CollectorConsumer".to_string(),
        },
        retries,
        component: ComponentInfo {
          name: "test".to_string(),
          type_name: "CollectorConsumer".to_string(),
        },
      };
      let action = consumer.handle_error(&error);
      prop_assert!(matches!(action, ErrorAction::Stop));
    }

    #[test]
    fn test_consumer_error_handling_skip(
      error_msg in prop::string::string_regex(".+").unwrap(),
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Skip,
        name: name.clone(),
      });
      let error = StreamError {
        source: Box::new(TestError(error_msg)),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: name.clone(),
          component_type: "TestProducer".to_string(),
        },
        component: ComponentInfo {
          name: name.clone(),
          type_name: "TestProducer".to_string(),
        },
        retries: 0,
      };
      prop_assert!(matches!(consumer.handle_error(&error), ErrorAction::Skip));
    }

    #[test]
    fn test_consumer_error_handling_retry(
      error_msg in prop::string::string_regex(".+").unwrap(),
      retry_count in 1..10usize,
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Retry(retry_count),
        name: name.clone(),
      });
      let error = StreamError {
        source: Box::new(TestError(error_msg)),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: name.clone(),
          component_type: "TestProducer".to_string(),
        },
        component: ComponentInfo {
          name: name.clone(),
          type_name: "TestProducer".to_string(),
        },
        retries: 0,
      };
      prop_assert!(matches!(consumer.handle_error(&error), ErrorAction::Retry));
    }

    #[test]
    fn test_consumer_error_handling_retry_exhausted(
      error_msg in prop::string::string_regex(".+").unwrap(),
      retry_limit in 1..10usize
    ) {
      let mut consumer = CollectorConsumer::<i32>::new();
      consumer = consumer.with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Retry(retry_limit),
        name: "test".to_string(),
      });
      let error = StreamError {
        source: Box::new(TestError(error_msg)),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "test".to_string(),
          component_type: "CollectorConsumer".to_string(),
        },
        retries: retry_limit,
        component: ComponentInfo {
          name: "test".to_string(),
          type_name: "CollectorConsumer".to_string(),
        },
      };
      let action = consumer.handle_error(&error);
      prop_assert!(matches!(action, ErrorAction::Stop));
    }
  }

  proptest! {
    #[test]
    fn test_consumer_component_info(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Stop,
        name: name.clone(),
      });
      let info = consumer.component_info();
      prop_assert_eq!(info.name, name);
      prop_assert_eq!(
        info.type_name,
        "streamweave::consumer::tests::CollectorConsumer<i32>"
      );
    }

    #[test]
    fn test_consumer_create_error_context(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Stop,
        name: name.clone(),
      });
      let context = consumer.create_error_context(None);
      prop_assert_eq!(context.component_name, name);
      prop_assert_eq!(
        context.component_type,
        "streamweave::consumer::tests::CollectorConsumer<i32>"
      );
      prop_assert!(context.item.is_none());
    }

    #[test]
    fn test_consumer_create_error_context_with_item(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      item in -1000..1000i32
    ) {
      let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Stop,
        name: name.clone(),
      });
      let context = consumer.create_error_context(Some(item));
      prop_assert_eq!(context.component_name, name);
      prop_assert_eq!(
        context.component_type,
        "streamweave::consumer::tests::CollectorConsumer<i32>"
      );
      prop_assert_eq!(context.item, Some(item));
    }
  }

  async fn test_configuration_changes_async(name1: String, name2: String, retry_count: usize) {
    let mut consumer = CollectorConsumer::<i32>::new();

    // First config
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: name1.clone(),
    });
    assert_eq!(consumer.config().name, name1);
    assert!(matches!(
      consumer.config().error_strategy,
      ErrorStrategy::Skip
    ));

    // Second config
    consumer = consumer.with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(retry_count),
      name: name2.clone(),
    });
    assert_eq!(consumer.config().name, name2);
    assert!(matches!(
      consumer.config().error_strategy,
      ErrorStrategy::Retry(r) if r == retry_count
    ));
  }

  #[test]
  fn test_multiple_configuration_changes() {
    proptest::proptest!(|(
      name1 in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      name2 in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      retry_count in 1..10usize
    )| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_configuration_changes_async(name1, name2, retry_count));
    });
  }

  async fn test_configuration_persistence_async(name: String, input1: Vec<i32>, input2: Vec<i32>) {
    let mut consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: name.clone(),
    });

    // First consume
    let stream1 = Box::pin(tokio_stream::iter(input1.clone()));
    consumer.consume(stream1).await;

    // Second consume - config should persist
    let stream2 = Box::pin(tokio_stream::iter(input2.clone()));
    consumer.consume(stream2).await;

    assert_eq!(consumer.config().name, name);
    assert!(matches!(
      consumer.config().error_strategy,
      ErrorStrategy::Skip
    ));
    let mut expected = input1;
    expected.extend(input2);
    assert_eq!(consumer.get_items().await, expected);
  }

  #[test]
  fn test_configuration_persistence() {
    proptest::proptest!(|(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      input1 in prop::collection::vec(-1000..1000i32, 0..50),
      input2 in prop::collection::vec(-1000..1000i32, 0..50)
    )| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_configuration_persistence_async(name, input1, input2));
    });
  }

  proptest! {
    #[test]
    fn test_custom_error_handler(
      error_msg in prop::string::string_regex(".+").unwrap(),
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::new_custom(|_| ErrorAction::Skip),
        name: name.clone(),
      });

      let error = StreamError {
        source: Box::new(std::io::Error::other(error_msg.clone())),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: name.clone(),
          component_type: "test".to_string(),
        },
        component: ComponentInfo {
          name: name.clone(),
          type_name: "test".to_string(),
        },
        retries: 0,
      };

      let action = consumer.handle_error(&error);
      prop_assert!(matches!(action, ErrorAction::Skip));
    }
  }

  #[derive(Debug)]
  struct DifferentError(String);
  impl std::fmt::Display for DifferentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }
  impl std::error::Error for DifferentError {}

  proptest! {
    #[test]
    fn test_different_error_types(
      error_msg in prop::string::string_regex(".+").unwrap(),
      retries in 0..10usize
    ) {
      let consumer = CollectorConsumer::<i32>::new();
      let error = StreamError {
        source: Box::new(DifferentError(error_msg)),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "CollectorConsumer".to_string(),
          component_type: "CollectorConsumer".to_string(),
        },
        retries,
        component: ComponentInfo {
          name: "test".to_string(),
          type_name: "CollectorConsumer".to_string(),
        },
      };
      let action = consumer.handle_error(&error);
      prop_assert!(matches!(action, ErrorAction::Stop));
    }
  }

  async fn test_concurrent_consumption_async(inputs: Vec<Vec<i32>>) {
    let consumer = Arc::new(tokio::sync::Mutex::new(CollectorConsumer::<i32>::new()));
    let mut handles = vec![];
    for input in inputs.iter() {
      let consumer = Arc::clone(&consumer);
      let input = input.clone();
      let handle = tokio::spawn(async move {
        let mut consumer = consumer.lock().await;
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
    let mut expected: Vec<i32> = inputs.iter().flatten().copied().collect();
    expected.sort();
    assert_eq!(collected, expected);
  }

  #[test]
  fn test_concurrent_consumption() {
    proptest::proptest!(|(inputs in prop::collection::vec(
      prop::collection::vec(-1000..1000i32, 1..10),
      1..10
    ))| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_concurrent_consumption_async(inputs));
    });
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

  async fn test_stream_backpressure_async(items: Vec<i32>) {
    let mut consumer = CollectorConsumer::<i32>::new();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));
    let handle = tokio::spawn(async move {
      consumer.consume(stream).await;
      consumer
    });
    // Send items
    for item in items.iter() {
      tx.send(*item).await.unwrap();
    }
    // Close the sender to signal stream completion
    drop(tx);
    let consumer = handle.await.unwrap();
    assert_eq!(consumer.get_items().await.len(), items.len());
  }

  #[test]
  fn test_stream_backpressure() {
    proptest::proptest!(|(items in prop::collection::vec(-1000..1000i32, 1..100))| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_stream_backpressure_async(items));
    });
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

  async fn test_thread_local_configuration_async(name1: String, name2: String, retry_count: usize) {
    let consumer1 = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: name1.clone(),
    });
    let consumer2 = CollectorConsumer::<i32>::new().with_config(ConsumerConfig {
      error_strategy: ErrorStrategy::Retry(retry_count),
      name: name2.clone(),
    });
    assert_eq!(consumer1.config().name, name1);
    assert_eq!(consumer2.config().name, name2);
    assert!(matches!(
      consumer1.config().error_strategy,
      ErrorStrategy::Skip
    ));
    assert!(matches!(
      consumer2.config().error_strategy,
      ErrorStrategy::Retry(r) if r == retry_count
    ));
  }

  #[test]
  fn test_thread_local_configuration() {
    proptest::proptest!(|(
      name1 in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      name2 in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      retry_count in 1..10usize
    )| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_thread_local_configuration_async(name1, name2, retry_count));
    });
  }
}
