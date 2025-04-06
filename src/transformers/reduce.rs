use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct ReduceTransformer<F, T, Acc> {
  reducer: F,
  initial: Acc,
  config: TransformerConfig,
  _phantom: std::marker::PhantomData<T>,
}

impl<F, T, Acc> ReduceTransformer<F, T, Acc>
where
  F:
    FnMut(Acc, T) -> Result<Acc, Box<dyn std::error::Error + Send + Sync>> + Send + Clone + 'static,
  T: Send + 'static,
  Acc: Send + Clone + 'static,
{
  pub fn new(reducer: F, initial: Acc) -> Self {
    Self {
      reducer,
      initial,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
  }
}

impl<F, T, Acc> crate::traits::error::Error for ReduceTransformer<F, T, Acc>
where
  F: Send + 'static,
  T: Send + 'static,
  Acc: Send + 'static,
{
  type Error = StreamError;
}

impl<F, T, Acc> Input for ReduceTransformer<F, T, Acc>
where
  F: Send + 'static,
  T: Send + 'static,
  Acc: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<F, T, Acc> Output for ReduceTransformer<F, T, Acc>
where
  F: Send + 'static,
  T: Send + 'static,
  Acc: Send + 'static,
{
  type Output = Acc;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<F, T, Acc> Transformer for ReduceTransformer<F, T, Acc>
where
  F:
    FnMut(Acc, T) -> Result<Acc, Box<dyn std::error::Error + Send + Sync>> + Send + Clone + 'static,
  T: Send + 'static,
  Acc: Send + Clone + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let reducer = self.reducer.clone();
    let initial = self.initial.clone();

    Box::pin(futures::stream::unfold(
      (input, Some(initial), reducer),
      |(mut input, mut acc, mut reducer)| async move {
        while let Some(item_result) = input.next().await {
          match item_result {
            Ok(item) => {
              if let Some(current_acc) = acc.take() {
                match reducer(current_acc, item) {
                  Ok(new_acc) => {
                    acc = Some(new_acc.clone());
                    return Some((Ok(new_acc), (input, acc, reducer)));
                  }
                  Err(e) => {
                    return Some((
                      Err(StreamError::new(
                        e,
                        ErrorContext {
                          timestamp: chrono::Utc::now(),
                          item: None,
                          stage: PipelineStage::Transformer,
                        },
                        ComponentInfo {
                          name: "reduce_transformer".to_string(),
                          type_name: std::any::type_name::<Self>().to_string(),
                        },
                      )),
                      (input, None, reducer),
                    ));
                  }
                }
              }
            }
            Err(e) => return Some((Err(e), (input, acc, reducer))),
          }
        }
        None
      },
    ))
  }

  fn config(&self) -> &TransformerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut TransformerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorStrategy::Retry(n),
      _ => ErrorStrategy::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Transformer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "reduce_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_reduce_transformer_sum() {
    let mut transformer = ReduceTransformer::new(|acc, x| Ok(acc + x), 0);
    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<i32> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 3, 6, 10, 15]);
  }

  #[tokio::test]
  async fn test_reduce_transformer_string_concat() {
    let mut transformer = ReduceTransformer::new(|acc: String, x: &str| Ok(acc + x), String::new());
    let input = vec!["a", "b", "c"];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<String> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec!["a".to_string(), "ab".to_string(), "abc".to_string()]
    );
  }

  #[tokio::test]
  async fn test_reduce_transformer_with_error() {
    let mut transformer = ReduceTransformer::new(
      |acc: i32, x: i32| {
        if x % 2 == 0 {
          Ok(acc + x)
        } else {
          Err("Odd numbers not allowed".into())
        }
      },
      0,
    );

    let input = vec![2, 3, 4, 5, 6];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result = transformer
      .transform(input_stream)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      StreamError { source, .. } => {
        assert_eq!(source.to_string(), "Odd numbers not allowed")
      }
    }
  }

  #[tokio::test]
  async fn test_reduce_transformer_custom_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Counter {
      count: i32,
      sum: i32,
    }

    let mut transformer = ReduceTransformer::new(
      |acc: Counter, x: i32| {
        Ok(Counter {
          count: acc.count + 1,
          sum: acc.sum + x,
        })
      },
      Counter { count: 0, sum: 0 },
    );

    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Counter> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec![
        Counter { count: 1, sum: 1 },
        Counter { count: 2, sum: 3 },
        Counter { count: 3, sum: 6 },
      ]
    );
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = ReduceTransformer::new(|acc, x| Ok(acc + x), 0)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      transformer.create_error_context(None),
      transformer.component_info(),
    );

    assert_eq!(transformer.handle_error(error), ErrorStrategy::Skip);
  }
}
