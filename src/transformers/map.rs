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

pub struct MapTransformer<F, I, O> {
  f: F,
  config: TransformerConfig,
  _phantom_i: std::marker::PhantomData<I>,
  _phantom_o: std::marker::PhantomData<O>,
}

impl<F, I, O> MapTransformer<F, I, O>
where
  F: FnMut(I) -> Result<O, Box<dyn std::error::Error + Send + Sync>> + Send + Clone + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  pub fn new(f: F) -> Self {
    Self {
      f,
      config: TransformerConfig::default(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
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

impl<F, I, O> Clone for MapTransformer<F, I, O>
where
  F: Clone,
{
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      config: self.config.clone(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }
}

impl<F, I, O> crate::traits::error::Error for MapTransformer<F, I, O>
where
  F: Send + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  type Error = StreamError;
}

impl<F, I, O> Input for MapTransformer<F, I, O>
where
  F: Send + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  type Input = I;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<F, I, O> Output for MapTransformer<F, I, O>
where
  F: Send + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  type Output = O;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<F, I, O> Transformer for MapTransformer<F, I, O>
where
  F: FnMut(I) -> Result<O, Box<dyn std::error::Error + Send + Sync>> + Send + Clone + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let f = self.f.clone();

    Box::pin(futures::stream::unfold(
      (input, f),
      |(mut input, mut f)| async move {
        match input.next().await {
          Some(result) => {
            let mapped = match result {
              Ok(item) => match f(item) {
                Ok(output) => Ok(output),
                Err(e) => Err(StreamError::new(
                  e,
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: None,
                    stage: PipelineStage::Transformer,
                  },
                  ComponentInfo {
                    name: "map_transformer".to_string(),
                    type_name: std::any::type_name::<Self>().to_string(),
                  },
                )),
              },
              Err(e) => Err(e),
            };
            Some((mapped, (input, f)))
          }
          None => None,
        }
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
        .unwrap_or_else(|| "map_transformer".to_string()),
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
  async fn test_map_transformer() {
    let mut transformer = MapTransformer::new(|x: i32| Ok(x * 2));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_map_transformer_type_conversion() {
    let mut transformer = MapTransformer::new(|x: i32| Ok(x.to_string()));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec!["1", "2", "3"]);
  }

  #[tokio::test]
  async fn test_map_transformer_reuse() {
    let mut transformer = MapTransformer::new(|x: i32| Ok(x * 2));

    // First transform
    let input1 = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer
      .transform(boxed_input1)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result1, vec![2, 4, 6]);

    // Second transform
    let input2 = stream::iter(vec![4, 5, 6].into_iter().map(Ok));
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer
      .transform(boxed_input2)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result2, vec![8, 10, 12]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = MapTransformer::new(|x: i32| Ok(x * 2))
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
