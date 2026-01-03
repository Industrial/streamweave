//! Tests for TempFileConsumer

use futures::{StreamExt, stream};
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Consumer, Input};
use streamweave_tempfile::TempFileConsumer;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_tempfile_consumer_basic() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();
  let mut consumer = TempFileConsumer::new(path);
  let input = stream::iter(vec!["test1".to_string(), "test2".to_string()]);
  let boxed_input = Box::pin(input);
  consumer.consume(boxed_input).await;
}

#[tokio::test]
async fn test_tempfile_consumer_empty() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();
  let mut consumer = TempFileConsumer::new(path);
  let input = stream::iter(Vec::<String>::new());
  let boxed_input = Box::pin(input);
  consumer.consume(boxed_input).await;
}

#[test]
fn test_tempfile_consumer_error_handling() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();
  let consumer = TempFileConsumer::new(path)
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("test_consumer".to_string());

  assert_eq!(
    consumer.config().error_strategy,
    ErrorStrategy::<String>::Skip
  );
  assert_eq!(consumer.config().name, "test_consumer");
}

#[test]
fn test_tempfile_consumer_input_trait() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();
  fn assert_input_trait(_consumer: TempFileConsumer)
  where
    TempFileConsumer: Input<Input = String>,
  {
  }

  let consumer = TempFileConsumer::new(path);
  assert_input_trait(consumer);
}
