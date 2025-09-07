use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::interval_producer::IntervalProducer;
use crate::traits::producer::{Producer, ProducerConfig};
use futures::stream;
use tokio::time;

impl Producer for IntervalProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let interval = self.interval;
    Box::pin(stream::unfold((), move |_| async move {
      time::sleep(interval).await;
      Some(((), ()))
    }))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<()>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<()> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<()> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<()>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<()>) -> ErrorContext<()> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "interval_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "interval_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use std::time::Duration;

  #[tokio::test]
  async fn test_interval_producer() {
    let mut producer = IntervalProducer::new(Duration::from_millis(10));
    let mut stream = producer.produce();

    // Take 3 items from the stream
    let start = time::Instant::now();
    let mut count = 0;
    while let Some(_) = stream.next().await {
      count += 1;
      if count >= 3 {
        break;
      }
    }
    let duration = start.elapsed();

    // Should take at least 20ms (2 intervals)
    assert!(duration >= Duration::from_millis(20));
    assert_eq!(count, 3);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let producer = IntervalProducer::new(Duration::from_millis(10))
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "IntervalProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "IntervalProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
