use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::flat_map::flat_map_transformer::FlatMapTransformer;
use crate::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<F, I, O> Transformer for FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let f = self.f.clone();
    Box::pin(input.flat_map(move |item| {
      let f = f.clone();
      futures::stream::iter(f(item))
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<I>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<I> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<I> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<I>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<I>) -> ErrorContext<I> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "flat_map_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_flat_map_basic() {
    let mut transformer = FlatMapTransformer::new(|x: i32| vec![x * 2, x * 3]);
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![2, 3, 4, 6, 6, 9]);
  }

  #[tokio::test]
  async fn test_flat_map_empty_input() {
    let mut transformer = FlatMapTransformer::new(|_: i32| Vec::<i32>::new());
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = FlatMapTransformer::new(|x: i32| vec![x * 2])
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
