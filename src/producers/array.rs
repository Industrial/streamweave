use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, StreamExt};
use std::cell::RefCell;
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;

thread_local! {
    static DEFAULT_CONFIG: RefCell<ProducerConfig> = RefCell::new(ProducerConfig::default());
}

#[derive(Debug)]
pub enum ArrayError {
  InvalidOperation(String),
}

impl fmt::Display for ArrayError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ArrayError::InvalidOperation(msg) => write!(f, "Array operation error: {}", msg),
    }
  }
}

impl StdError for ArrayError {}

pub struct ArrayProducer<T, const N: usize> {
  array: [T; N],
  config: ProducerConfig,
}

impl<T: Clone, const N: usize> ArrayProducer<T, N> {
  pub fn new(array: [T; N]) -> Self {
    Self {
      array,
      config: ProducerConfig::default(),
    }
  }

  pub fn from_slice(slice: &[T]) -> Option<Self>
  where
    T: Copy,
  {
    if slice.len() != N {
      return None;
    }
    let mut arr = std::mem::MaybeUninit::<[T; N]>::uninit();
    let ptr = arr.as_mut_ptr() as *mut T;
    unsafe {
      for (i, &item) in slice.iter().enumerate() {
        ptr.add(i).write(item);
      }
      Some(Self::new(arr.assume_init()))
    }
  }

  pub fn len(&self) -> usize {
    N
  }

  pub fn is_empty(&self) -> bool {
    N == 0
  }
}

impl<T: Clone + Send + 'static, const N: usize> Error for ArrayProducer<T, N> {
  type Error = ArrayError;
}

impl<T: Clone + Send + 'static, const N: usize> Output for ArrayProducer<T, N> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, ArrayError>> + Send>>;
}

impl<T: Clone + Send + 'static, const N: usize> Producer for ArrayProducer<T, N> {
  fn produce(&mut self) -> Self::OutputStream {
    let items = self.array.clone();
    Box::pin(futures::stream::iter(items.into_iter().map(Ok)))
  }

  fn config(&self) -> &ProducerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
    &mut self.config
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_array_producer() {
    let mut producer = ArrayProducer::new([1, 2, 3]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_array_producer_empty() {
    let mut producer = ArrayProducer::<i32, 0>::new([]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_array_producer_from_slice_success() {
    let slice = &[1, 2, 3];
    let mut producer = ArrayProducer::<_, 3>::from_slice(slice).unwrap();
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_array_producer_from_slice_failure() {
    let slice = &[1, 2, 3];
    assert!(ArrayProducer::<_, 4>::from_slice(slice).is_none());
  }

  #[tokio::test]
  async fn test_array_producer_multiple_calls() {
    let mut producer = ArrayProducer::new([1, 2, 3]);

    // First call
    let stream = producer.produce();
    let result1: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec![1, 2, 3]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_len_and_is_empty() {
    let producer: ArrayProducer<i32, 3> = ArrayProducer::new([1, 2, 3]);
    assert_eq!(producer.len(), 3);
    assert!(!producer.is_empty());

    let empty_producer: ArrayProducer<i32, 0> = ArrayProducer::new([]);
    assert_eq!(empty_producer.len(), 0);
    assert!(empty_producer.is_empty());
  }

  #[test]
  fn test_error_handling_stop() {
    let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    let error = StreamError {
      source: Box::new(ArrayError::InvalidOperation("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ArrayProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Stop));
  }

  #[test]
  fn test_error_handling_skip() {
    let mut producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    producer = producer.with_error_strategy(ErrorStrategy::Skip);
    let error = StreamError {
      source: Box::new(ArrayError::InvalidOperation("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ArrayProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Skip));
  }

  #[test]
  fn test_error_handling_retry() {
    let mut producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    producer = producer.with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(ArrayError::InvalidOperation("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ArrayProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Retry(3)));
  }

  #[test]
  fn test_error_handling_retry_exhausted() {
    let mut producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    producer = producer.with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(ArrayError::InvalidOperation("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      retries: 3,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ArrayProducer".to_string(),
      },
    };

    let strategy = producer.handle_error(error);
    assert!(matches!(strategy, ErrorStrategy::Stop));
  }

  #[test]
  fn test_component_info() {
    let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]).with_name("test_producer".to_string());
    let info = producer.component_info();
    assert_eq!(info.name, "test_producer");
    assert_eq!(info.type_name, "ArrayProducer");
  }

  #[test]
  fn test_create_error_context() {
    let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    let context = producer.create_error_context(None);
    assert!(matches!(context.stage, PipelineStage::Producer));
    assert!(context.item.is_none());
  }

  #[test]
  fn test_create_error_context_with_item() {
    let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    let item = Box::new(42) as Box<dyn std::any::Any + Send>;
    let context = producer.create_error_context(Some(item));
    assert!(matches!(context.stage, PipelineStage::Producer));
    assert!(context.item.is_some());
  }
}
