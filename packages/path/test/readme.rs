//! Integration tests for README examples
//!
//! These tests verify that the code examples in the README.md file work correctly.

use futures::StreamExt;
use streamweave::{Input, Output, Transformer};
use streamweave_path::{FileNameTransformer, NormalizePathTransformer, ParentPathTransformer};

#[tokio::test]
async fn test_readme_file_name_transformer_example() {
  // Example from README: Extract Filenames
  let mut transformer = FileNameTransformer::new();
  let input = futures::stream::iter(vec![
    "/path/to/file.txt".to_string(),
    "/another/path/document.pdf".to_string(),
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
async fn test_readme_parent_transformer_example() {
  // Example from README: Get Parent Directories
  let mut transformer = ParentPathTransformer::new();
  let input = futures::stream::iter(vec!["/path/to/file.txt".to_string()]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let result = output_stream.next().await;
  assert_eq!(result, Some("/path/to".to_string()));
}

#[tokio::test]
async fn test_readme_normalize_transformer_example() {
  // Example from README: Normalize Paths
  let mut transformer = NormalizePathTransformer::new();
  let input = futures::stream::iter(vec!["/path/./to/../file.txt".to_string()]);
  let input_stream = Box::pin(input);
  let mut output_stream = transformer.transform(input_stream).await;

  let result = output_stream.next().await;
  assert!(result.is_some());
  // Normalized path should contain "file.txt"
  assert!(result.unwrap().contains("file.txt"));
}

#[tokio::test]
async fn test_readme_chain_transformations_example() {
  // Example from README: Chain Path Transformations
  let mut parent_transformer = ParentPathTransformer::new();
  let mut file_name_transformer = FileNameTransformer::new();

  let input = futures::stream::iter(vec!["/path/to/file.txt".to_string()]);
  let input_stream = Box::pin(input);

  // First transformation: get parent
  let mut intermediate_stream = parent_transformer.transform(input_stream).await;
  let parent_result = intermediate_stream.next().await;
  assert_eq!(parent_result, Some("/path/to".to_string()));

  // Second transformation: get filename from parent
  let input2 = futures::stream::iter(vec!["/path/to".to_string()]);
  let input_stream2 = Box::pin(input2);
  let mut final_stream = file_name_transformer.transform(input_stream2).await;
  let filename_result = final_stream.next().await;
  assert_eq!(filename_result, Some("to".to_string()));
}

#[tokio::test]
async fn test_readme_configuration_example() {
  // Example from README: Transformer Configuration
  let transformer = FileNameTransformer::new().with_name("filename-extractor".to_string());

  assert_eq!(
    transformer.config().name(),
    Some(&"filename-extractor".to_string())
  );
}
