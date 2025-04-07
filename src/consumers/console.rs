use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{consumer::Consumer, input::Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct ConsoleConsumer<T> {
  config: ConsumerConfig<T>,
}

impl<T> ConsoleConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug + std::fmt::Display,
{
  pub fn new() -> Self {
    Self {
      config: ConsumerConfig::default(),
    }
  }
}

impl<T> crate::traits::error::Error for ConsoleConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug + std::fmt::Display,
{
  type Error = StreamError<T>;
}

impl<T> Input for ConsoleConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug + std::fmt::Display,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError<T>>> + Send>>;
}

#[async_trait]
impl<T> Consumer for ConsoleConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug + std::fmt::Display,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), StreamError<T>> {
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          println!("{}", value);
        }
        Err(e) => {
          let action = self.handle_error(&e);
          match action {
            ErrorAction::Stop => return Err(e),
            ErrorAction::Skip => continue,
            ErrorAction::Retry => {
              // Retry logic would go here
              return Err(e);
            }
          }
        }
      }
    }
    Ok(())
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.get_config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(_) => ErrorAction::Retry,
      ErrorStrategy::Custom(_) => ErrorAction::Skip,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer(self.component_info().name),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    let config = self.get_config();
    ComponentInfo {
      name: config.name,
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_console_consumer_integers() {
    let mut consumer = ConsoleConsumer::<i32>::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_strings() {
    let mut consumer = ConsoleConsumer::<String>::new();
    let input = stream::iter(vec![Ok("hello".to_string()), Ok("world".to_string())]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_floats() {
    let mut consumer = ConsoleConsumer::<f64>::new();
    let input = stream::iter(vec![Ok(1.1), Ok(2.2), Ok(3.3)]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_custom_type() {
    #[derive(Debug)]
    struct CustomType(i32);

    impl std::fmt::Display for CustomType {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Custom({})", self.0)
      }
    }

    let mut consumer = ConsoleConsumer::<CustomType>::new();
    let input = stream::iter(vec![
      Ok(CustomType(1)),
      Ok(CustomType(2)),
      Ok(CustomType(3)),
    ]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_reuse() {
    let mut consumer = ConsoleConsumer::<i32>::new();

    // First consumption
    let input1 = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let boxed_input1 = Box::pin(input1);
    assert!(consumer.consume(boxed_input1).await.is_ok());

    // Second consumption - should work fine
    let input2 = stream::iter(vec![Ok(4), Ok(5), Ok(6)]);
    let boxed_input2 = Box::pin(input2);
    assert!(consumer.consume(boxed_input2).await.is_ok());
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut consumer = ConsoleConsumer::<i32>::new();
    let error_stream = Box::pin(stream::iter(vec![
      Ok(1),
      Err(StreamError::new(
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Consumer,
        },
        ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      )),
      Ok(3),
    ]));

    let result = consumer.consume(error_stream).await;
    assert!(result.is_err());
  }
}
