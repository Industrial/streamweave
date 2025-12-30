//! Tests for StdoutConsumer

use futures::{StreamExt, stream};
use streamweave::{Consumer, Input};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_stdio::StdoutConsumer;

#[tokio::test]
async fn test_stdout_consumer_basic() {
  let mut consumer = StdoutConsumer::<String>::new();
  let input = stream::iter(vec!["test1".to_string(), "test2".to_string()]);
  let boxed_input = Box::pin(input);
  consumer.consume(boxed_input).await;
}

#[tokio::test]
async fn test_stdout_consumer_empty() {
  let mut consumer = StdoutConsumer::<String>::new();
  let input = stream::iter(Vec::<String>::new());
  let boxed_input = Box::pin(input);
  consumer.consume(boxed_input).await;
}

#[test]
fn test_stdout_consumer_error_handling() {
  let consumer = StdoutConsumer::<String>::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("test_consumer".to_string());

  assert_eq!(
    consumer.config().error_strategy,
    ErrorStrategy::<String>::Skip
  );
  assert_eq!(consumer.config().name, "test_consumer");
}

#[test]
fn test_stdout_consumer_input_trait() {
  fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
    _consumer: StdoutConsumer<T>,
  ) where
    StdoutConsumer<T>: Input<Input = T>,
  {
  }

  let consumer = StdoutConsumer::<String>::new();
  assert_input_trait(consumer);
}
