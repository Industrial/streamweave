//! Tests for TempFileProducer

use futures::StreamExt;
use std::io::Write;
use streamweave::Producer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_tempfile::TempFileProducer;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_tempfile_producer() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  writeln!(file, "line 2").unwrap();
  writeln!(file, "line 3").unwrap();
  file.flush().unwrap();
  let path = file.path().to_path_buf();
  let mut producer = TempFileProducer::new(path);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert_eq!(result, vec!["line 1", "line 2", "line 3"]);
  drop(file);
}

#[tokio::test]
async fn test_empty_file() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();
  let mut producer = TempFileProducer::new(path);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert!(result.is_empty());
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();
  let producer = TempFileProducer::new(path)
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("test_producer".to_string());

  let config = producer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
  assert_eq!(config.name(), Some("test_producer".to_string()));
}
