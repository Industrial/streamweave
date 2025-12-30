//! Tests for FileConsumer

use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::{Consumer, Input};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_file::FileConsumer;
use tempfile::NamedTempFile;
use tokio::fs as tokio_fs;
use tokio::fs::File;

async fn test_file_consumer_basic_async(input_data: Vec<String>) {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path.clone());

  let expected_contents = input_data.concat();
  let input = stream::iter(input_data.clone());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;

  // Use async file reading to ensure all writes are complete
  let contents = tokio_fs::read_to_string(path).await.unwrap();
  assert_eq!(contents, expected_contents);
  drop(temp_file);
}

proptest! {
  #[test]
  fn test_file_consumer_basic(input_data in prop::collection::vec(any::<String>(), 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_file_consumer_basic_async(input_data));
  }
}

async fn test_file_consumer_empty_input_async() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path.clone());

  let input = stream::iter(Vec::<String>::new());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;

  // Use async file reading to ensure all writes are complete
  let contents = tokio_fs::read_to_string(path).await.unwrap();
  assert_eq!(contents, "");
  drop(temp_file);
}

proptest! {
  #[test]
  fn test_file_consumer_empty_input(_ in prop::num::u8::ANY) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_file_consumer_empty_input_async());
  }
}

proptest! {
  #[test]
  fn test_error_handling_strategies(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path)
      .with_error_strategy(ErrorStrategy::<String>::Skip)
      .with_name(name.clone());

    prop_assert_eq!(
      &consumer.config().error_strategy,
      &ErrorStrategy::<String>::Skip
    );
    prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
  }
}

#[tokio::test]
async fn test_set_config_impl() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path);
  let new_config = streamweave::ConsumerConfig::<String> {
    name: "test_consumer".to_string(),
    error_strategy: ErrorStrategy::<String>::Retry(5),
  };

  consumer.set_config_impl(new_config.clone());
  assert_eq!(consumer.get_config_impl().name, new_config.name);
  assert_eq!(
    consumer.get_config_impl().error_strategy,
    new_config.error_strategy
  );
}

#[tokio::test]
async fn test_get_config_mut_impl() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path);
  let config_mut = consumer.get_config_mut_impl();
  config_mut.name = "mutated_name".to_string();
  assert_eq!(consumer.get_config_impl().name, "mutated_name");
}

#[tokio::test]
async fn test_handle_error_stop() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Stop);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_handle_error_skip() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Skip);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_handle_error_retry_within_limit() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 3;
  assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_handle_error_retry_exceeds_limit() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 5;
  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_create_error_context() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path).with_name("test_consumer".to_string());
  let context = consumer.create_error_context(Some("test_item".to_string()));
  assert_eq!(context.item, Some("test_item".to_string()));
  assert_eq!(context.component_name, "test_consumer");
  assert!(context.timestamp <= chrono::Utc::now());
}

#[tokio::test]
async fn test_create_error_context_no_item() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path).with_name("test_consumer".to_string());
  let context = consumer.create_error_context(None);
  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "test_consumer");
}

#[tokio::test]
async fn test_component_info() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path).with_name("test_consumer".to_string());
  let info = consumer.component_info();
  assert_eq!(info.name, "test_consumer");
  assert_eq!(info.type_name, std::any::type_name::<FileConsumer>());
}

#[tokio::test]
async fn test_component_info_default_name() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path);
  let info = consumer.component_info();
  assert_eq!(info.name, "");
  assert_eq!(info.type_name, std::any::type_name::<FileConsumer>());
}

#[tokio::test]
async fn test_consume_with_existing_file() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path.clone());

  // Create file first
  let _file = File::create(&path).await.unwrap();
  consumer.file = Some(_file);

  let input = stream::iter(vec!["test1".to_string(), "test2".to_string()]);
  let boxed_input = Box::pin(input);
  consumer.consume(boxed_input).await;

  let contents = tokio_fs::read_to_string(path).await.unwrap();
  assert_eq!(contents, "test1test2");
}

// Input trait tests

async fn test_file_consumer_input_stream_send_bound_async(data: Vec<String>) {
  // Test that the InputStream implements Send bound for async usage
  let _consumer = FileConsumer::new("test.txt".to_string());

  // Create a stream that matches the InputStream type
  let stream: Pin<Box<dyn futures::Stream<Item = String> + Send>> =
    Box::pin(stream::iter(data.clone()));

  // Test that we can spawn this stream in a task (requires Send)
  let handle = tokio::spawn(async move {
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, data);
  });

  handle.await.unwrap();
}

proptest! {
  #[test]
  fn test_file_consumer_input_trait_implementation(path in "[a-zA-Z0-9_./-]+\\.txt") {
    // Test that FileConsumer implements Input trait correctly
    fn assert_input_trait(_consumer: FileConsumer)
    where
      FileConsumer: Input<Input = String>,
    {
      // This function compiles only if FileConsumer implements Input<Input = String>
    }

    let consumer = FileConsumer::new(path);
    assert_input_trait(consumer);
  }

  #[test]
  fn test_file_consumer_input_type_constraints(path in "[a-zA-Z0-9_./-]+\\.txt") {
    // Test that the Input type is correctly set to String
    fn get_input_type(_consumer: FileConsumer) -> std::marker::PhantomData<String>
    where
      FileConsumer: Input<Input = String>,
    {
      std::marker::PhantomData
    }

    let consumer = FileConsumer::new(path);
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to String
  }

  #[test]
  fn test_file_consumer_input_stream_type(path in "[a-zA-Z0-9_./-]+\\.txt") {
    // Test that the InputStream type is correctly constrained
    fn create_input_stream(_consumer: FileConsumer) -> Pin<Box<dyn futures::Stream<Item = String> + Send>>
    where
      FileConsumer: Input<Input = String, InputStream = Pin<Box<dyn futures::Stream<Item = String> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let consumer = FileConsumer::new(path);
    let _stream = create_input_stream(consumer);
    // This compiles only if the InputStream type is correctly constrained
  }

  #[test]
  fn test_file_consumer_input_stream_send_bound(data in prop::collection::vec(any::<String>(), 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_file_consumer_input_stream_send_bound_async(data));
  }

  #[test]
  fn test_file_consumer_input_trait_bounds(path in "[a-zA-Z0-9_./-]+\\.txt") {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds(consumer: FileConsumer)
    where
      FileConsumer: Input,
    {
      // Test that the consumer can be used as Input
      let _consumer = consumer;
    }

    test_trait_bounds(FileConsumer::new(path));
  }

  #[test]
  fn test_file_consumer_input_static_lifetime(path in "[a-zA-Z0-9_./-]+\\.txt") {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime(consumer: FileConsumer)
    where
      FileConsumer: Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // This should compile because FileConsumer has 'static lifetime
    test_static_lifetime(FileConsumer::new(path));
  }

  #[test]
  fn test_file_consumer_input_stream_compatibility(data in prop::collection::vec(any::<String>(), 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
      // Test that streams can be created and used with the Input trait
      let _consumer = FileConsumer::new("test.txt".to_string());

      // Create a stream that matches the expected InputStream type
      let stream: Pin<Box<dyn futures::Stream<Item = String> + Send>> = Box::pin(stream::iter(data.clone()));

      // Test that we can collect from the stream
      let result: Vec<String> = stream.collect().await;
      assert_eq!(result, data);
    });
  }

  #[test]
  fn test_file_consumer_input_trait_object_safety(path in "[a-zA-Z0-9_./-]+\\.txt") {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
      // This function can accept any type that implements Input
    }

    // Test with FileConsumer instance
    process_input(FileConsumer::new(path));
  }
}

#[tokio::test]
async fn test_file_consumer_handle_error_custom_strategy() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let custom_handler = |error: &StreamError<String>| {
    if error.retries < 3 {
      ErrorAction::Retry
    } else {
      ErrorAction::Skip
    }
  };
  let consumer = FileConsumer::new(path)
    .with_error_strategy(ErrorStrategy::<String>::new_custom(custom_handler));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 2;
  assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
  error.retries = 3;
  assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_file_consumer_create_error_context_default_name() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let consumer = FileConsumer::new(path);
  let context = consumer.create_error_context(Some("test_item".to_string()));
  assert_eq!(context.item, Some("test_item".to_string()));
  assert_eq!(context.component_name, "");
}

#[tokio::test]
async fn test_file_consumer_large_data() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path.clone());

  let large_data: Vec<String> = (0..1000).map(|i| format!("line {}\n", i)).collect();
  let input = stream::iter(large_data.clone());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;

  let contents = tokio_fs::read_to_string(path).await.unwrap();
  let expected = large_data.concat();
  assert_eq!(contents, expected);
}

#[tokio::test]
async fn test_file_consumer_multiple_consumes() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path.clone());

  // First consume
  let input1 = stream::iter(vec!["first".to_string(), "batch".to_string()]);
  consumer.consume(Box::pin(input1)).await;

  // Second consume (should append or overwrite depending on implementation)
  let input2 = stream::iter(vec!["second".to_string(), "batch".to_string()]);
  consumer.consume(Box::pin(input2)).await;

  let contents = tokio_fs::read_to_string(path).await.unwrap();
  // File should contain both batches (no newlines between items)
  assert!(contents.contains("first"));
  assert!(contents.contains("second"));
}

#[tokio::test]
async fn test_file_consumer_special_characters() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path.clone());

  let special_data = vec![
    "line with\nnewline".to_string(),
    "line with\ttab".to_string(),
    "line with\"quotes".to_string(),
    "line with'apostrophe".to_string(),
    "line with\\backslash".to_string(),
  ];
  let input = stream::iter(special_data.clone());
  consumer.consume(Box::pin(input)).await;

  let contents = tokio_fs::read_to_string(path).await.unwrap();
  let expected = special_data.concat();
  assert_eq!(contents, expected);
}

#[tokio::test]
async fn test_file_consumer_unicode() {
  let temp_file = NamedTempFile::new().unwrap();
  let path = temp_file.path().to_str().unwrap().to_string();
  let mut consumer = FileConsumer::new(path.clone());

  let unicode_data = vec![
    "Hello 世界".to_string(),
    "Здравствуй".to_string(),
    "こんにちは".to_string(),
    "مرحبا".to_string(),
  ];
  let input = stream::iter(unicode_data.clone());
  consumer.consume(Box::pin(input)).await;

  let contents = tokio_fs::read_to_string(path).await.unwrap();
  let expected = unicode_data.concat();
  assert_eq!(contents, expected);
}
