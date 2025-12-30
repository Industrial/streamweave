//! Tests for DirectoryConsumer

use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::{Consumer, Input};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_fs::DirectoryConsumer;
use tempfile::TempDir;
use tokio::fs as tokio_fs;

async fn test_directory_consumer_basic_async(paths: Vec<String>) {
  let temp_dir = TempDir::new().unwrap();
  let base_path = temp_dir.path().to_str().unwrap().to_string();

  let mut consumer = DirectoryConsumer::new();

  // Create full paths relative to temp directory
  let full_paths: Vec<String> = paths
    .iter()
    .map(|p| format!("{}/{}", base_path, p))
    .collect();

  let input = stream::iter(full_paths.clone());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;

  // Verify all directories were created
  for path in &full_paths {
    let metadata = tokio_fs::metadata(path).await;
    assert!(metadata.is_ok(), "Directory should exist: {}", path);
    assert!(
      metadata.unwrap().is_dir(),
      "Path should be a directory: {}",
      path
    );
  }
}

proptest! {
  #[test]
  fn test_directory_consumer_basic(paths in prop::collection::vec("[a-zA-Z0-9_/]+", 0..10)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_directory_consumer_basic_async(paths));
  }
}

#[tokio::test]
async fn test_directory_consumer_empty_input() {
  let mut consumer = DirectoryConsumer::new();

  let input = stream::iter(Vec::<String>::new());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  // Should complete without error
}

#[tokio::test]
async fn test_directory_consumer_nested_paths() {
  let temp_dir = TempDir::new().unwrap();
  let base_path = temp_dir.path().to_str().unwrap().to_string();

  let mut consumer = DirectoryConsumer::new();

  let paths = vec![
    format!("{}/level1/level2/level3", base_path),
    format!("{}/level1/level2", base_path),
    format!("{}/level1", base_path),
  ];

  let input = stream::iter(paths.clone());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;

  // Verify all nested directories were created
  for path in &paths {
    let metadata = tokio_fs::metadata(path).await;
    assert!(metadata.is_ok(), "Directory should exist: {}", path);
    assert!(
      metadata.unwrap().is_dir(),
      "Path should be a directory: {}",
      path
    );
  }
}

#[tokio::test]
async fn test_directory_consumer_with_name() {
  let consumer = DirectoryConsumer::new().with_name("my_consumer".to_string());
  assert_eq!(consumer.config().name.as_str(), "my_consumer");
}

#[tokio::test]
async fn test_directory_consumer_with_error_strategy() {
  let consumer = DirectoryConsumer::new()
    .with_error_strategy(ErrorStrategy::<String>::Skip)
    .with_name("test_consumer".to_string());

  assert_eq!(
    consumer.config().error_strategy,
    ErrorStrategy::<String>::Skip
  );
  assert_eq!(consumer.config().name.as_str(), "test_consumer");
}

#[tokio::test]
async fn test_directory_consumer_default() {
  let consumer = DirectoryConsumer::default();
  assert_eq!(consumer.config().name.as_str(), "");
}

#[tokio::test]
async fn test_directory_consumer_set_config_impl() {
  let mut consumer = DirectoryConsumer::new();
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
async fn test_directory_consumer_get_config_mut_impl() {
  let mut consumer = DirectoryConsumer::new();
  let config_mut = consumer.get_config_mut_impl();
  config_mut.name = "mutated_name".to_string();
  assert_eq!(consumer.get_config_impl().name, "mutated_name");
}

#[tokio::test]
async fn test_directory_consumer_handle_error_stop() {
  let consumer = DirectoryConsumer::new().with_error_strategy(ErrorStrategy::<String>::Stop);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_directory_consumer_handle_error_skip() {
  let consumer = DirectoryConsumer::new().with_error_strategy(ErrorStrategy::<String>::Skip);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_directory_consumer_handle_error_retry_within_limit() {
  let consumer = DirectoryConsumer::new().with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 3;
  assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_directory_consumer_handle_error_retry_exceeds_limit() {
  let consumer = DirectoryConsumer::new().with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 5;
  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_directory_consumer_handle_error_custom_strategy() {
  let custom_handler = |error: &StreamError<String>| {
    if error.retries < 3 {
      ErrorAction::Retry
    } else {
      ErrorAction::Skip
    }
  };
  let consumer = DirectoryConsumer::new()
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
async fn test_directory_consumer_create_error_context() {
  let consumer = DirectoryConsumer::new().with_name("test_consumer".to_string());
  let context = consumer.create_error_context(Some("test_item".to_string()));
  assert_eq!(context.item, Some("test_item".to_string()));
  assert_eq!(context.component_name, "test_consumer");
  assert!(context.timestamp <= chrono::Utc::now());
}

#[tokio::test]
async fn test_directory_consumer_create_error_context_no_item() {
  let consumer = DirectoryConsumer::new().with_name("test_consumer".to_string());
  let context = consumer.create_error_context(None);
  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "test_consumer");
}

#[tokio::test]
async fn test_directory_consumer_create_error_context_default_name() {
  let consumer = DirectoryConsumer::new();
  let context = consumer.create_error_context(Some("test_item".to_string()));
  assert_eq!(context.component_name, "");
}

#[tokio::test]
async fn test_directory_consumer_component_info() {
  let consumer = DirectoryConsumer::new().with_name("test_consumer".to_string());
  let info = consumer.component_info();
  assert_eq!(info.name, "test_consumer");
  assert_eq!(info.type_name, std::any::type_name::<DirectoryConsumer>());
}

#[tokio::test]
async fn test_directory_consumer_component_info_default_name() {
  let consumer = DirectoryConsumer::new();
  let info = consumer.component_info();
  assert_eq!(info.name, "");
  assert_eq!(info.type_name, std::any::type_name::<DirectoryConsumer>());
}

#[tokio::test]
async fn test_directory_consumer_existing_directory() {
  let temp_dir = TempDir::new().unwrap();
  let base_path = temp_dir.path().to_str().unwrap().to_string();
  let path = format!("{}/existing_dir", base_path);

  // Create directory first
  tokio_fs::create_dir_all(&path).await.unwrap();

  let mut consumer = DirectoryConsumer::new();
  let input = stream::iter(vec![path.clone()]);
  consumer.consume(Box::pin(input)).await;

  // Directory should still exist (create_dir_all is idempotent)
  let metadata = tokio_fs::metadata(&path).await;
  assert!(metadata.is_ok());
  assert!(metadata.unwrap().is_dir());
}

#[tokio::test]
async fn test_directory_consumer_multiple_consumes() {
  let temp_dir = TempDir::new().unwrap();
  let base_path = temp_dir.path().to_str().unwrap().to_string();

  let mut consumer = DirectoryConsumer::new();

  // First consume
  let paths1 = vec![format!("{}/dir1", base_path), format!("{}/dir2", base_path)];
  let input1 = stream::iter(paths1.clone());
  consumer.consume(Box::pin(input1)).await;

  // Second consume
  let paths2 = vec![format!("{}/dir3", base_path), format!("{}/dir4", base_path)];
  let input2 = stream::iter(paths2.clone());
  consumer.consume(Box::pin(input2)).await;

  // Verify all directories were created
  for path in paths1.iter().chain(paths2.iter()) {
    let metadata = tokio_fs::metadata(path).await;
    assert!(metadata.is_ok(), "Directory should exist: {}", path);
  }
}

// Input trait tests

async fn test_directory_consumer_input_stream_send_bound_async(data: Vec<String>) {
  let _consumer = DirectoryConsumer::new();

  let stream: Pin<Box<dyn futures::Stream<Item = String> + Send>> =
    Box::pin(stream::iter(data.clone()));

  let handle = tokio::spawn(async move {
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, data);
  });

  handle.await.unwrap();
}

proptest! {
  #[test]
  fn test_directory_consumer_input_trait_implementation(path in "[a-zA-Z0-9_./-]+") {
    fn assert_input_trait(_consumer: DirectoryConsumer)
    where
      DirectoryConsumer: Input<Input = String>,
    {
    }

    let consumer = DirectoryConsumer::new();
    assert_input_trait(consumer);
  }

  #[test]
  fn test_directory_consumer_input_type_constraints(path in "[a-zA-Z0-9_./-]+") {
    fn get_input_type(_consumer: DirectoryConsumer) -> std::marker::PhantomData<String>
    where
      DirectoryConsumer: Input<Input = String>,
    {
      std::marker::PhantomData
    }

    let consumer = DirectoryConsumer::new();
    let _phantom = get_input_type(consumer);
  }

  #[test]
  fn test_directory_consumer_input_stream_type(path in "[a-zA-Z0-9_./-]+") {
    fn create_input_stream(_consumer: DirectoryConsumer) -> Pin<Box<dyn futures::Stream<Item = String> + Send>>
    where
      DirectoryConsumer: Input<Input = String, InputStream = Pin<Box<dyn futures::Stream<Item = String> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let consumer = DirectoryConsumer::new();
    let _stream = create_input_stream(consumer);
  }

  #[test]
  fn test_directory_consumer_input_stream_send_bound(data in prop::collection::vec(any::<String>(), 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_directory_consumer_input_stream_send_bound_async(data));
  }

  #[test]
  fn test_directory_consumer_input_trait_bounds(path in "[a-zA-Z0-9_./-]+") {
    fn test_trait_bounds(consumer: DirectoryConsumer)
    where
      DirectoryConsumer: Input,
    {
      let _consumer = consumer;
    }

    test_trait_bounds(DirectoryConsumer::new());
  }

  #[test]
  fn test_directory_consumer_input_static_lifetime(path in "[a-zA-Z0-9_./-]+") {
    fn test_static_lifetime(consumer: DirectoryConsumer)
    where
      DirectoryConsumer: Input,
    {
      let _consumer = consumer;
    }

    test_static_lifetime(DirectoryConsumer::new());
  }

  #[test]
  fn test_directory_consumer_input_stream_compatibility(data in prop::collection::vec(any::<String>(), 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
      let _consumer = DirectoryConsumer::new();

      let stream: Pin<Box<dyn futures::Stream<Item = String> + Send>> = Box::pin(stream::iter(data.clone()));

      let result: Vec<String> = stream.collect().await;
      assert_eq!(result, data);
    });
  }

  #[test]
  fn test_directory_consumer_input_trait_object_safety(path in "[a-zA-Z0-9_./-]+") {
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
    }

    process_input(DirectoryConsumer::new());
  }
}
