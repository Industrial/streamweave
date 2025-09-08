use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::transformers::map::MapTransformer;
use crate::traits::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<F, I, O> Transformer for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let f = self.f.clone();
    Box::pin(input.map(f))
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
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<I>) -> ErrorContext<I> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "map_transformer".to_string()),
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
  async fn test_map_transformer() {
    let mut transformer = MapTransformer::new(|x: i32| x * 2);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_map_transformer_type_conversion() {
    let mut transformer = MapTransformer::new(|x: i32| x.to_string());
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec!["1", "2", "3"]);
  }

  #[tokio::test]
  async fn test_map_transformer_reuse() {
    let mut transformer = MapTransformer::new(|x: i32| x * 2);

    // First transform
    let input1 = stream::iter(vec![1, 2, 3]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![2, 4, 6]);

    // Second transform
    let input2 = stream::iter(vec![4, 5, 6]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![8, 10, 12]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = MapTransformer::new(|x: i32| x * 2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
