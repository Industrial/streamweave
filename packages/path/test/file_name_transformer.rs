use futures::StreamExt;
use streamweave::{Input, Output, Transformer};
use streamweave_path::FileNameTransformer;

#[tokio::test]
async fn test_file_name_transformer_basic() {
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/to/file.txt".to_string(),
    "/another/path/document.pdf".to_string(),
    "simple.txt".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec!["file.txt", "document.pdf", "simple.txt"]);
}

#[tokio::test]
async fn test_file_name_transformer_empty_path() {
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec!["".to_string()]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let result = output_stream.next().await;
  assert_eq!(result, Some("".to_string()));
}

#[tokio::test]
async fn test_file_name_transformer_root_path() {
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec!["/".to_string()]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let result = output_stream.next().await;
  assert_eq!(result, Some("".to_string()));
}

#[tokio::test]
async fn test_file_name_transformer_with_name() {
  let transformer = FileNameTransformer::new().with_name("test-transformer".to_string());
  assert_eq!(
    transformer.config().name(),
    Some(&"test-transformer".to_string())
  );
}

#[tokio::test]
async fn test_file_name_transformer_default() {
  let transformer = FileNameTransformer::default();
  assert_eq!(transformer.config().name(), None);
}

#[tokio::test]
async fn test_file_name_transformer_clone() {
  let transformer1 = FileNameTransformer::new().with_name("original".to_string());
  let transformer2 = transformer1.clone();
  assert_eq!(transformer1.config().name(), transformer2.config().name());
}

#[tokio::test]
async fn test_file_name_transformer_with_error_strategy() {
  use streamweave::error::ErrorStrategy;
  let transformer = FileNameTransformer::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_file_name_transformer_handle_error_stop() {
  use std::error::Error;
  use std::fmt;
  use streamweave::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = FileNameTransformer::new().with_error_strategy(ErrorStrategy::Stop);

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    streamweave::error::ErrorAction::Stop
  ));
}

#[tokio::test]
async fn test_file_name_transformer_handle_error_skip() {
  use streamweave::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = FileNameTransformer::new().with_error_strategy(ErrorStrategy::Skip);

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    streamweave::error::ErrorAction::Skip
  ));
}

#[tokio::test]
async fn test_file_name_transformer_handle_error_retry() {
  use streamweave::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = FileNameTransformer::new().with_error_strategy(ErrorStrategy::Retry(3));

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 1,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    streamweave::error::ErrorAction::Retry
  ));
}

#[tokio::test]
async fn test_file_name_transformer_handle_error_retry_exceeded() {
  use streamweave::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = FileNameTransformer::new().with_error_strategy(ErrorStrategy::Retry(3));

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 3,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    streamweave::error::ErrorAction::Stop
  ));
}

#[tokio::test]
async fn test_file_name_transformer_handle_error_custom() {
  use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

  let transformer = FileNameTransformer::new()
    .with_error_strategy(ErrorStrategy::new_custom(|_| ErrorAction::Skip));

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    streamweave::error::ErrorAction::Skip
  ));
}

#[tokio::test]
async fn test_file_name_transformer_create_error_context() {
  let transformer = FileNameTransformer::new().with_name("test-transformer".to_string());
  let context = transformer.create_error_context(Some("test_path".to_string()));

  assert_eq!(context.component_name, "test-transformer");
  assert_eq!(context.item, Some("test_path".to_string()));
  assert!(context.component_type.contains("FileNameTransformer"));
}

#[tokio::test]
async fn test_file_name_transformer_create_error_context_default_name() {
  let transformer = FileNameTransformer::new();
  let context = transformer.create_error_context(None);

  assert_eq!(context.component_name, "file_name_transformer");
  assert_eq!(context.item, None);
}

#[tokio::test]
async fn test_file_name_transformer_component_info() {
  let transformer = FileNameTransformer::new().with_name("test-transformer".to_string());
  let info = transformer.component_info();

  assert_eq!(info.name, "test-transformer");
  assert!(info.type_name.contains("FileNameTransformer"));
}

#[tokio::test]
async fn test_file_name_transformer_component_info_default_name() {
  let transformer = FileNameTransformer::new();
  let info = transformer.component_info();

  assert_eq!(info.name, "file_name_transformer");
}

#[tokio::test]
async fn test_file_name_transformer_config_methods() {
  use streamweave::TransformerConfig;

  let mut transformer = FileNameTransformer::new();
  let new_config = TransformerConfig::default();

  transformer.set_config_impl(new_config.clone());
  assert_eq!(transformer.get_config_impl(), &new_config);
  assert_eq!(transformer.get_config_mut_impl(), &mut new_config);
}

#[tokio::test]
async fn test_file_name_transformer_windows_paths() {
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec![
    r"C:\path\to\file.txt".to_string(),
    r"C:\another\path\document.pdf".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec!["file.txt", "document.pdf"]);
}

#[tokio::test]
async fn test_file_name_transformer_special_characters() {
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/to/file with spaces.txt".to_string(),
    "/path/to/file-with-dashes.txt".to_string(),
    "/path/to/file_with_underscores.txt".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(
    results,
    vec![
      "file with spaces.txt",
      "file-with-dashes.txt",
      "file_with_underscores.txt"
    ]
  );
}

#[tokio::test]
async fn test_file_name_transformer_no_extension() {
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/to/file".to_string(),
    "/path/to/README".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec!["file", "README"]);
}

#[tokio::test]
async fn test_file_name_transformer_unicode() {
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/to/файл.txt".to_string(),
    "/path/to/文件.pdf".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec!["файл.txt", "文件.pdf"]);
}

// Helper for error tests
#[derive(Debug)]
struct StringError(String);

impl fmt::Display for StringError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Error for StringError {}
