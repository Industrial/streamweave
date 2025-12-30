use futures::StreamExt;
use streamweave::Transformer;
use streamweave_path::NormalizePathTransformer;

#[tokio::test]
async fn test_normalize_path_transformer_basic() {
  let mut transformer = NormalizePathTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/to/./file.txt".to_string(),
    "/another/../path/document.pdf".to_string(),
    "simple.txt".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Normalized paths should have redundant components removed
  assert_eq!(results.len(), 3);
  assert!(results[0].contains("file.txt"));
  assert!(results[1].contains("document.pdf"));
  assert_eq!(results[2], "simple.txt");
}

#[tokio::test]
async fn test_normalize_path_transformer_empty_path() {
  let mut transformer = NormalizePathTransformer::new();
  let input = futures::stream::iter(vec!["".to_string()]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let result = output_stream.next().await;
  assert_eq!(result, Some("".to_string()));
}

#[tokio::test]
async fn test_normalize_path_transformer_with_name() {
  let transformer = NormalizePathTransformer::new().with_name("test-transformer".to_string());
  assert_eq!(
    transformer.config().name(),
    Some(&"test-transformer".to_string())
  );
}

#[tokio::test]
async fn test_normalize_path_transformer_default() {
  let transformer = NormalizePathTransformer::default();
  assert_eq!(transformer.config().name(), None);
}

#[tokio::test]
async fn test_normalize_path_transformer_clone() {
  let transformer1 = NormalizePathTransformer::new().with_name("original".to_string());
  let transformer2 = transformer1.clone();
  assert_eq!(transformer1.config().name(), transformer2.config().name());
}

#[tokio::test]
async fn test_normalize_path_transformer_with_error_strategy() {
  use streamweave_error::ErrorStrategy;
  let transformer = NormalizePathTransformer::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_normalize_path_transformer_handle_error_stop() {
  use std::error::Error;
  use std::fmt;
  use streamweave_error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = NormalizePathTransformer::new().with_error_strategy(ErrorStrategy::Stop);

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
    streamweave_error::ErrorAction::Stop
  ));
}

#[tokio::test]
async fn test_normalize_path_transformer_handle_error_skip() {
  use streamweave_error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = NormalizePathTransformer::new().with_error_strategy(ErrorStrategy::Skip);

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
    streamweave_error::ErrorAction::Skip
  ));
}

#[tokio::test]
async fn test_normalize_path_transformer_handle_error_retry() {
  use streamweave_error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = NormalizePathTransformer::new().with_error_strategy(ErrorStrategy::Retry(3));

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
    streamweave_error::ErrorAction::Retry
  ));
}

#[tokio::test]
async fn test_normalize_path_transformer_handle_error_retry_exceeded() {
  use streamweave_error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let transformer = NormalizePathTransformer::new().with_error_strategy(ErrorStrategy::Retry(3));

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
    streamweave_error::ErrorAction::Stop
  ));
}

#[tokio::test]
async fn test_normalize_path_transformer_handle_error_custom() {
  use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

  let transformer = NormalizePathTransformer::new()
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
    streamweave_error::ErrorAction::Skip
  ));
}

#[tokio::test]
async fn test_normalize_path_transformer_create_error_context() {
  let transformer = NormalizePathTransformer::new().with_name("test-transformer".to_string());
  let context = transformer.create_error_context(Some("test_path".to_string()));

  assert_eq!(context.component_name, "test-transformer");
  assert_eq!(context.item, Some("test_path".to_string()));
  assert!(context.component_type.contains("NormalizePathTransformer"));
}

#[tokio::test]
async fn test_normalize_path_transformer_create_error_context_default_name() {
  let transformer = NormalizePathTransformer::new();
  let context = transformer.create_error_context(None);

  assert_eq!(context.component_name, "normalize_path_transformer");
  assert_eq!(context.item, None);
}

#[tokio::test]
async fn test_normalize_path_transformer_component_info() {
  let transformer = NormalizePathTransformer::new().with_name("test-transformer".to_string());
  let info = transformer.component_info();

  assert_eq!(info.name, "test-transformer");
  assert!(info.type_name.contains("NormalizePathTransformer"));
}

#[tokio::test]
async fn test_normalize_path_transformer_component_info_default_name() {
  let transformer = NormalizePathTransformer::new();
  let info = transformer.component_info();

  assert_eq!(info.name, "normalize_path_transformer");
}

#[tokio::test]
async fn test_normalize_path_transformer_config_methods() {
  use streamweave::TransformerConfig;

  let mut transformer = NormalizePathTransformer::new();
  let new_config = TransformerConfig::default();

  transformer.set_config_impl(new_config.clone());
  assert_eq!(transformer.get_config_impl(), &new_config);
  assert_eq!(transformer.get_config_mut_impl(), &mut new_config);
}

#[tokio::test]
async fn test_normalize_path_transformer_dot_components() {
  let mut transformer = NormalizePathTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/./to/./file.txt".to_string(),
    "./relative/path.txt".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 2);
  assert!(results[0].contains("file.txt"));
  assert!(results[1].contains("path.txt"));
}

#[tokio::test]
async fn test_normalize_path_transformer_dotdot_components() {
  let mut transformer = NormalizePathTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/to/../file.txt".to_string(),
    "/another/../path/document.pdf".to_string(),
  ]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 2);
  assert!(results[0].contains("file.txt"));
  assert!(results[1].contains("document.pdf"));
}

// Helper for error tests
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct StringError(String);

impl fmt::Display for StringError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Error for StringError {}
