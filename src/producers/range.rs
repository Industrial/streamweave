use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use num_traits::Num;
use std::pin::Pin;

pub struct RangeProducer<T>
where
  T: Num + Copy + Clone + Send + PartialOrd + 'static,
{
  start: T,
  end: T,
  step: T,
  config: ProducerConfig<T>,
}

impl<T> RangeProducer<T>
where
  T: Num + Copy + Clone + Send + PartialOrd + 'static,
{
  pub fn new(start: T, end: T, step: T) -> Self {
    Self {
      start,
      end,
      step,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Output for RangeProducer<T>
where
  T: Num + Copy + Clone + Send + PartialOrd + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Producer for RangeProducer<T>
where
  T: Num + Copy + Clone + Send + PartialOrd + 'static,
{
  fn produce(&mut self) -> Self::OutputStream {
    if self.start >= self.end || self.step <= T::zero() {
      return Box::pin(stream::empty());
    }

    let start = self.start;
    let end = self.end;
    let step = self.step;

    let stream = stream::unfold(start, move |current| async move {
      if current >= end {
        None
      } else {
        let next = current + step;
        Some((current, next))
      }
    });

    Box::pin(stream)
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "range_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_range_producer_integers() {
    let mut producer = RangeProducer::new(0i32, 5i32, 1i32);
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![0, 1, 2, 3, 4]);
  }

  #[tokio::test]
  async fn test_range_producer_float() {
    let mut producer = RangeProducer::new(0.0f64, 1.0f64, 0.2f64);
    let stream = producer.produce();
    let result: Vec<f64> = stream.collect().await;
    assert!((result[0] - 0.0).abs() < f64::EPSILON);
    assert!((result[1] - 0.2).abs() < f64::EPSILON);
    assert!((result[2] - 0.4).abs() < f64::EPSILON);
    assert!((result[3] - 0.6).abs() < f64::EPSILON);
    assert!((result[4] - 0.8).abs() < f64::EPSILON);
  }

  #[tokio::test]
  async fn test_range_producer_custom_step() {
    let mut producer = RangeProducer::new(0, 10, 2);
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![0, 2, 4, 6, 8]);
  }

  #[tokio::test]
  async fn test_invalid_range() {
    let mut producer = RangeProducer::new(5, 0, 1);
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_invalid_step() {
    let mut producer = RangeProducer::new(0, 5, 0);
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert!(result.is_empty());

    let mut producer = RangeProducer::new(0, 5, -1);
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = RangeProducer::new(0, 5, 1)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "test error",
      )),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RangeProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
