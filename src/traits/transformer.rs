use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::input::Input;
use crate::traits::output::Output;
use std::any::Any;
use std::cell::RefCell;

thread_local! {
    static DEFAULT_CONFIG: RefCell<TransformerConfig> = RefCell::new(TransformerConfig {
        error_strategy: ErrorStrategy::Stop,
        name: None,
    });
}

#[derive(Debug, Clone)]
pub struct TransformerConfig {
  pub error_strategy: ErrorStrategy,
  pub name: Option<String>,
}

pub trait Transformer: Input + Output {
  fn transform(&mut self, input: <Self as Input>::InputStream) -> <Self as Output>::OutputStream;

  fn with_config(&self, config: TransformerConfig) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  fn set_config(&mut self, config: TransformerConfig) {
    DEFAULT_CONFIG.with(|c| *c.borrow_mut() = config);
  }

  fn get_config(&self) -> TransformerConfig {
    DEFAULT_CONFIG.with(|c| c.borrow().clone())
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.get_config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    let config = self.get_config();
    ComponentInfo {
      name: config.name.unwrap_or_else(|| "transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Transformer(self.component_info().name),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::error::Error;
  use futures::Stream;
  use futures::{StreamExt, stream};
  use std::error::Error as StdError;
  use std::fmt;
  use std::pin::Pin;
  use std::sync::{Arc, Mutex};

  // Test error type
  #[derive(Debug, Clone)]
  struct TestError(String);

  impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl StdError for TestError {}

  // Mock transformer that doubles numbers
  #[derive(Clone)]
  struct DoubleTransformer;

  impl Error for DoubleTransformer {
    type Error = TestError;
  }

  impl Input for DoubleTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Output for DoubleTransformer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Transformer for DoubleTransformer {
    fn transform(&mut self, input: <Self as Input>::InputStream) -> <Self as Output>::OutputStream {
      Box::pin(input.map(|r| r.map(|x| x * 2)))
    }
  }

  // Mock transformer that converts numbers to strings
  #[derive(Clone)]
  struct ToStringTransformer;

  impl Error for ToStringTransformer {
    type Error = TestError;
  }

  impl Input for ToStringTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Output for ToStringTransformer {
    type Output = String;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<String, TestError>> + Send>>;
  }

  impl Transformer for ToStringTransformer {
    fn transform(&mut self, input: <Self as Input>::InputStream) -> <Self as Output>::OutputStream {
      Box::pin(input.map(|r| r.map(|x| x.to_string())))
    }
  }

  #[tokio::test]
  async fn test_double_transformer() {
    let mut transformer = DoubleTransformer;
    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<i32> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, vec![2, 4, 6, 8, 10]);
  }

  #[tokio::test]
  async fn test_double_transformer_empty_stream() {
    let mut transformer = DoubleTransformer;
    let input: Vec<i32> = vec![];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<i32> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_string_transformer() {
    let mut transformer = ToStringTransformer;
    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<String> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, vec!["1", "2", "3"]);
  }

  #[tokio::test]
  async fn test_chained_transformers() {
    let mut double_transformer = DoubleTransformer;
    let mut string_transformer = ToStringTransformer;

    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    // Chain transformers: numbers -> doubled -> strings
    let doubled_stream = double_transformer.transform(input_stream);
    let final_stream = string_transformer.transform(doubled_stream);

    let result: Vec<String> = final_stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["2", "4", "6"]);
  }

  #[tokio::test]
  async fn test_transformer_error_propagation() {
    let mut double_transformer = DoubleTransformer;
    let input = vec![Ok(1), Err(TestError("test error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input));

    let output_stream = double_transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].as_ref().unwrap(), &2);
    assert!(result[1].is_err());
    assert_eq!(result[2].as_ref().unwrap(), &6);
  }

  #[tokio::test]
  async fn test_multiple_configuration_changes() {
    let mut transformer = DoubleTransformer;

    // First config
    transformer = transformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: Some("first".to_string()),
    });
    assert_eq!(transformer.get_config().name, Some("first".to_string()));
    assert!(matches!(
      transformer.get_config().error_strategy,
      ErrorStrategy::Skip
    ));

    // Second config
    transformer = transformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: Some("second".to_string()),
    });
    assert_eq!(transformer.get_config().name, Some("second".to_string()));
    assert!(matches!(
      transformer.get_config().error_strategy,
      ErrorStrategy::Retry(3)
    ));
  }

  #[tokio::test]
  async fn test_configuration_persistence() {
    let mut transformer = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: Some("persistent".to_string()),
    });

    // First transform
    let input1 = vec![1, 2, 3];
    let input_stream1 = Box::pin(stream::iter(input1).map(Ok));
    let output_stream1 = transformer.transform(input_stream1);
    let result1: Vec<i32> = output_stream1.map(|r| r.unwrap()).collect().await;

    // Second transform - config should persist
    let input2 = vec![4, 5, 6];
    let input_stream2 = Box::pin(stream::iter(input2).map(Ok));
    let output_stream2 = transformer.transform(input_stream2);
    let result2: Vec<i32> = output_stream2.map(|r| r.unwrap()).collect().await;

    assert_eq!(
      transformer.get_config().name,
      Some("persistent".to_string())
    );
    assert!(matches!(
      transformer.get_config().error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[tokio::test]
  async fn test_concurrent_transformation() {
    let transformer = Arc::new(Mutex::new(DoubleTransformer));
    let mut handles = vec![];

    for i in 0..3 {
      let transformer = Arc::clone(&transformer);
      let handle = tokio::spawn(async move {
        let input = vec![i as i32 * 3 + 1, i as i32 * 3 + 2, i as i32 * 3 + 3];
        let input_stream = Box::pin(stream::iter(input).map(Ok));

        // Lock, transform, and unlock before await
        let output_stream = {
          let mut transformer = transformer.lock().unwrap();
          transformer.transform(input_stream)
        };

        let result: Vec<i32> = output_stream.map(|r| r.unwrap()).collect().await;
        result
      });
      handles.push(handle);
    }

    for (i, handle) in handles.into_iter().enumerate() {
      let result = handle.await.unwrap();
      let expected = vec![
        (i as i32 * 3 + 1) * 2,
        (i as i32 * 3 + 2) * 2,
        (i as i32 * 3 + 3) * 2,
      ];
      assert_eq!(result, expected);
    }
  }

  #[tokio::test]
  async fn test_stream_cancellation() {
    let mut transformer = DoubleTransformer;
    let (tx, rx) = tokio::sync::oneshot::channel();

    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input).map(Ok));
    let stream = transformer.transform(input_stream);

    let handle = tokio::spawn(async move {
      let result: Vec<Result<i32, TestError>> = stream.collect().await;
      tx.send(result).unwrap();
    });

    // Cancel the stream
    handle.abort();

    // Verify the result
    let result = rx.await.unwrap();
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_stream_backpressure() {
    let mut transformer = DoubleTransformer;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input).map(Ok));
    let stream = transformer.transform(input_stream);

    let handle = tokio::spawn(async move {
      let mut stream = stream;
      while let Some(item) = stream.next().await {
        tx.send(item).await.unwrap();
      }
    });

    // Consume items with delay to create backpressure
    for _ in 0..3 {
      let item = rx.recv().await.unwrap();
      assert!(item.is_ok());
      tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    handle.await.unwrap();
  }

  #[tokio::test]
  async fn test_stream_timeout() {
    let mut transformer = DoubleTransformer;
    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input).map(Ok));
    let stream = transformer.transform(input_stream);

    let handle = tokio::spawn(async move {
      tokio::time::timeout(
        std::time::Duration::from_millis(100),
        stream.collect::<Vec<_>>(),
      )
      .await
    });

    let result = handle.await.unwrap();
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_thread_local_configuration() {
    let transformer1 = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: Some("transformer1".to_string()),
    });

    let transformer2 = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: Some("transformer2".to_string()),
    });

    assert_eq!(
      transformer1.get_config().name,
      Some("transformer1".to_string())
    );
    assert_eq!(
      transformer2.get_config().name,
      Some("transformer2".to_string())
    );
    assert!(matches!(
      transformer1.get_config().error_strategy,
      ErrorStrategy::Skip
    ));
    assert!(matches!(
      transformer2.get_config().error_strategy,
      ErrorStrategy::Retry(3)
    ));
  }

  #[tokio::test]
  async fn test_transformer_with_different_error_types() {
    #[derive(Debug)]
    struct DifferentError(String);

    impl fmt::Display for DifferentError {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Different error: {}", self.0)
      }
    }

    impl StdError for DifferentError {}

    let mut transformer = DoubleTransformer;
    let error = StreamError {
      source: Box::new(DifferentError("different error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Transformer(transformer.component_info().name),
      },
      retries: 0,
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "DoubleTransformer".to_string(),
      },
    };

    let action = transformer.handle_error(&error);
    assert!(matches!(action, ErrorAction::Stop));
  }

  #[tokio::test]
  async fn test_error_strategy_stop() {
    let mut transformer = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Stop,
      name: Some("stop_strategy".to_string()),
    });

    let input = vec![Ok(1), Err(TestError("error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input));
    let output_stream = transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].as_ref().unwrap(), &2);
  }

  #[tokio::test]
  async fn test_error_strategy_skip() {
    let mut transformer = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Skip,
      name: Some("skip_strategy".to_string()),
    });

    let input = vec![Ok(1), Err(TestError("error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input));
    let output_stream = transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].as_ref().unwrap(), &2);
    assert_eq!(result[1].as_ref().unwrap(), &6);
  }

  #[tokio::test]
  async fn test_error_strategy_retry() {
    let mut transformer = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Retry(3),
      name: Some("retry_strategy".to_string()),
    });

    let mut retry_count = 0;
    let input = vec![Ok(1), Err(TestError("error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input).map(move |r| {
      if r.is_err() {
        retry_count += 1;
      }
      r
    }));
    let output_stream = transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(retry_count, 1);
    assert_eq!(result.len(), 3);
  }

  #[tokio::test]
  async fn test_error_strategy_custom() {
    let mut transformer = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Custom(Box::new(|error| {
        if error.retries < 2 {
          ErrorAction::Retry
        } else {
          ErrorAction::Skip
        }
      })),
      name: Some("custom_strategy".to_string()),
    });

    let input = vec![Ok(1), Err(TestError("error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input));
    let output_stream = transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].as_ref().unwrap(), &2);
    assert_eq!(result[1].as_ref().unwrap(), &6);
  }

  #[tokio::test]
  async fn test_max_retry_limit() {
    let mut transformer = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Retry(2),
      name: Some("max_retry".to_string()),
    });

    let mut retry_count = 0;
    let input = vec![Ok(1), Err(TestError("error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input).map(move |r| {
      if r.is_err() {
        retry_count += 1;
      }
      r
    }));
    let output_stream = transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(retry_count, 2);
    assert_eq!(result.len(), 2);
  }

  #[tokio::test]
  async fn test_error_context_with_different_items() {
    let mut transformer = DoubleTransformer;

    // Test with string item
    let string_item = Box::new("test".to_string()) as Box<dyn Any + Send>;
    let context1 = transformer.create_error_context(Some(string_item));
    assert!(context1.item.is_some());

    // Test with numeric item
    let num_item = Box::new(42) as Box<dyn Any + Send>;
    let context2 = transformer.create_error_context(Some(num_item));
    assert!(context2.item.is_some());

    // Test with no item
    let context3 = transformer.create_error_context(None);
    assert!(context3.item.is_none());
  }

  #[tokio::test]
  async fn test_custom_error_handler() {
    let mut transformer = DoubleTransformer.with_config(TransformerConfig {
      error_strategy: ErrorStrategy::Custom(Box::new(|error| {
        if error.retries < 2 {
          ErrorAction::Retry
        } else {
          ErrorAction::Skip
        }
      })),
      name: Some("custom_handler".to_string()),
    });

    let mut retry_count = 0;
    let input = vec![Ok(1), Err(TestError("error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input).map(move |r| {
      if r.is_err() {
        retry_count += 1;
      }
      r
    }));
    let output_stream = transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(retry_count, 2);
    assert_eq!(result.len(), 2);
  }

  #[tokio::test]
  async fn test_large_stream_handling() {
    let mut transformer = DoubleTransformer;
    let input: Vec<i32> = (0..10000).collect();
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<i32> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result.len(), 10000);
    assert_eq!(result[0], 0);
    assert_eq!(result[9999], 19998);
  }

  #[tokio::test]
  async fn test_memory_usage() {
    let mut transformer = DoubleTransformer;
    let input: Vec<i32> = (0..1000000).collect();
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<i32> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result.len(), 1000000);
    assert_eq!(result[0], 0);
    assert_eq!(result[999999], 1999998);
  }
}
