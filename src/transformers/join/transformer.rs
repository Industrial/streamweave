use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::join::join_transformer::{JoinState, JoinTransformer};
use async_trait::async_trait;
use futures::StreamExt;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
#[allow(dead_code)]
impl<L, R, K, LF, RF> Transformer for JoinTransformer<L, R, K, LF, RF>
where
  L: std::fmt::Debug + Clone + Send + Sync + 'static,
  R: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: Hash + Eq + Clone + Send + Sync + 'static,
  LF: Fn(&L) -> K + Clone + Send + Sync + 'static,
  RF: Fn(&R) -> K + Clone + Send + Sync + 'static,
{
  type InputPorts = (L,);
  type OutputPorts = (crate::transformers::join::join_transformer::JoinResult<L, R>,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let left_key_fn = self.left_key_fn.clone();
    let right_key_fn = self.right_key_fn.clone();
    let right_stream = self.right_stream.take();

    Box::pin(async_stream::stream! {
      let state = Arc::new(RwLock::new(JoinState::<L, R, K>::default()));

      // If we have a right stream, process both concurrently
      if let Some(right_stream) = right_stream {
        let state_for_right = Arc::clone(&state);
        let right_key_fn_clone = right_key_fn.clone();

        // Spawn task to process right stream
        let right_handle = tokio::spawn(async move {
          let mut right_stream = right_stream;
          let mut results = Vec::new();

          while let Some(item) = right_stream.next().await {
            let key = right_key_fn_clone(&item);
            let mut state = state_for_right.write().await;
            results.extend(state.add_right(key, item));
          }

          results
        });

        // Process left stream
        let mut input = input;
        while let Some(item) = input.next().await {
          let key = left_key_fn(&item);
          let mut state = state.write().await;
          for result in state.add_left(key, item) {
            yield result;
          }
        }

        // Wait for right stream processing and yield any remaining results
        if let Ok(results) = right_handle.await {
          for result in results {
            yield result;
          }
        }
      } else {
        // No right stream, just buffer left elements (useful for later joining)
        let mut input = input;
        while let Some(item) = input.next().await {
          let key = left_key_fn(&item);
          let mut state = state.write().await;
          for result in state.add_left(key, item) {
            yield result;
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<L>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<L> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<L> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<L>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<L>) -> ErrorContext<L> {
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
        .name
        .clone()
        .unwrap_or_else(|| "join".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::transformers::join::join_transformer::JoinResult;
  use futures::stream;

  #[tokio::test]
  async fn test_join_basic() {
    let left_data = vec![1, 2, 3];
    let right_data = vec!["1", "2", "3"];

    let left_stream = stream::iter(left_data);
    let right_stream: std::pin::Pin<Box<dyn futures::Stream<Item = &str> + Send>> =
      Box::pin(stream::iter(right_data));

    let mut transformer =
      JoinTransformer::new(|x: &i32| *x, |s: &&str| s.parse::<i32>().unwrap_or(0))
        .with_right_stream(right_stream);

    let output_stream = transformer.transform(Box::pin(left_stream));
    let results: Vec<JoinResult<i32, &str>> = output_stream.collect().await;

    // Each left element should match with its corresponding right element
    assert!(!results.is_empty());
  }

  #[tokio::test]
  async fn test_join_no_right_stream() {
    let left_data = vec![1, 2, 3];
    let left_stream = stream::iter(left_data);

    let mut transformer: JoinTransformer<i32, String, i32, _, _> =
      JoinTransformer::new(|x: &i32| *x, |s: &String| s.len() as i32);

    let output_stream = transformer.transform(Box::pin(left_stream));
    let results: Vec<JoinResult<i32, String>> = output_stream.collect().await;

    // No right stream, so no join results
    assert!(results.is_empty());
  }

  #[tokio::test]
  async fn test_join_multiple_matches() {
    // Multiple left elements with same key
    let left_data = vec![1, 1, 2];
    let right_data = vec!["a", "b"];

    let left_stream = stream::iter(left_data);
    let right_stream: std::pin::Pin<Box<dyn futures::Stream<Item = &str> + Send>> =
      Box::pin(stream::iter(right_data));

    let mut transformer =
      JoinTransformer::new(|x: &i32| *x, |s: &&str| if *s == "a" { 1 } else { 2 })
        .with_right_stream(right_stream);

    let output_stream = transformer.transform(Box::pin(left_stream));
    let results: Vec<JoinResult<i32, &str>> = output_stream.collect().await;

    // Should have matches where keys align
    assert!(!results.is_empty());
  }

  #[test]
  fn test_join_component_info() {
    let transformer: JoinTransformer<i32, String, i32, _, _> =
      JoinTransformer::new(|x: &i32| *x, |s: &String| s.len() as i32)
        .with_name("custom_join".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "custom_join");
  }
}
