use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use num_traits::Num;
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;

pub struct RangeProducer<T> {
  start: T,
  end: T,
  step: T,
}

impl<T> RangeProducer<T>
where
  T: Num + Copy + Send + PartialOrd + 'static,
{
  pub fn new(start: T, end: T, step: T) -> Self {
    Self { start, end, step }
  }
}

#[derive(Debug)]
pub enum RangeError {
  InvalidRange,
}

impl fmt::Display for RangeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      RangeError::InvalidRange => write!(f, "Invalid range specified"),
    }
  }
}

impl StdError for RangeError {}

impl<T> Error for RangeProducer<T>
where
  T: Num + Copy + Send + Sync + PartialOrd + 'static,
{
  type Error = RangeError;
}

impl<T> Output for RangeProducer<T>
where
  T: Num + Copy + Send + Sync + PartialOrd + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, RangeError>> + Send>>;
}

impl<T> Producer for RangeProducer<T>
where
  T: Num + Copy + Send + Sync + PartialOrd + 'static,
{
  fn produce(&mut self) -> Self::OutputStream {
    if self.start >= self.end || self.step <= T::zero() {
      return Box::pin(futures::stream::once(async {
        Err(RangeError::InvalidRange)
      }));
    }

    let start = self.start;
    let end = self.end;
    let step = self.step;

    let stream = futures::stream::unfold(start, move |current| async move {
      if current >= end {
        None
      } else {
        let next = current + step;
        Some((Ok(current), next))
      }
    });

    Box::pin(stream)
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
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec![0, 1, 2, 3, 4]);
  }

  #[tokio::test]
  async fn test_range_producer_float() {
    let mut producer = RangeProducer::new(0.0f64, 1.0f64, 0.2f64);
    let stream = producer.produce();
    let result: Vec<f64> = stream.map(|r| r.unwrap()).collect().await;
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
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec![0, 2, 4, 6, 8]);
  }

  #[tokio::test]
  async fn test_invalid_range() {
    let mut producer = RangeProducer::new(5, 0, 1);
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(RangeError::InvalidRange)));
  }

  #[tokio::test]
  async fn test_invalid_step() {
    let mut producer = RangeProducer::new(0, 5, 0);
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(RangeError::InvalidRange)));

    let mut producer = RangeProducer::new(0, 5, -1);
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(RangeError::InvalidRange)));
  }
}
