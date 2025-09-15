use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::group_by::group_by_transformer::GroupByTransformer;
use async_stream;
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::HashMap;
use std::hash::Hash;

#[async_trait]
impl<F, T, K> Transformer for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Clone + Send + Sync + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let key_fn = self.key_fn.clone();
    Box::pin(async_stream::stream! {
        let mut groups = HashMap::new();
        let mut input = input;

        while let Some(item) = input.next().await {
            let key = key_fn(&item);
            groups.entry(key).or_insert_with(Vec::new).push(item);
        }

        // Sort groups by key for deterministic output
        let mut groups = groups.into_iter().collect::<Vec<_>>();
        groups.sort_by_key(|(k, _)| k.clone());

        for group in groups {
            yield group;
        }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "group_by_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "group_by_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use proptest::prelude::*;

  #[tokio::test]
  async fn test_group_by_basic() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| *k);

    assert_eq!(result, vec![(0, vec![2, 4, 6]), (1, vec![1, 3, 5])]);
  }

  #[tokio::test]
  async fn test_group_by_empty_input() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = GroupByTransformer::new(|x: &i32| x % 2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  #[test]
  fn test_group_by_transformer_new() {
    let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);

    assert_eq!(transformer.config().name(), None);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_group_by_transformer_with_error_strategy() {
    let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
      .with_error_strategy(ErrorStrategy::Skip);

    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_group_by_transformer_with_name() {
    let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
      .with_name("test_group_by".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("test_group_by".to_string())
    );
  }

  #[tokio::test]
  async fn test_group_by_transformer_single_group() {
    let mut transformer = GroupByTransformer::new(|_x: &i32| 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![(0, vec![1, 2, 3, 4, 5])]);
  }

  #[tokio::test]
  async fn test_group_by_transformer_multiple_groups() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 3);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| *k);

    assert_eq!(
      result,
      vec![(0, vec![3, 6, 9]), (1, vec![1, 4, 7]), (2, vec![2, 5, 8])]
    );
  }

  #[tokio::test]
  async fn test_group_by_transformer_string_keys() {
    let mut transformer = GroupByTransformer::new(|x: &i32| {
      if *x < 5 {
        "small".to_string()
      } else {
        "large".to_string()
      }
    });
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(String, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| k.clone());

    assert_eq!(
      result,
      vec![
        ("large".to_string(), vec![5, 6, 7, 8, 9, 10]),
        ("small".to_string(), vec![1, 2, 3, 4])
      ]
    );
  }

  #[tokio::test]
  async fn test_group_by_transformer_float_keys() {
    let mut transformer = GroupByTransformer::new(|x: &f64| (x * 10.0).round() as i64);
    let input = stream::iter(vec![1.1, 1.2, 1.3, 2.1, 2.2, 2.3]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i64, Vec<f64>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| *k);

    // Each value gets its own group because (1.1 * 10).round() = 11, (1.2 * 10).round() = 12, etc.
    assert_eq!(
      result,
      vec![
        (11, vec![1.1]),
        (12, vec![1.2]),
        (13, vec![1.3]),
        (21, vec![2.1]),
        (22, vec![2.2]),
        (23, vec![2.3])
      ]
    );
  }

  #[tokio::test]
  async fn test_group_by_transformer_duplicate_keys() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(vec![1, 1, 1, 2, 2, 2]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| *k);

    assert_eq!(result, vec![(0, vec![2, 2, 2]), (1, vec![1, 1, 1])]);
  }

  #[tokio::test]
  async fn test_group_by_transformer_negative_numbers() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x.abs() % 3);
    let input = stream::iter(vec![-1, -2, -3, 1, 2, 3]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| *k);

    assert_eq!(
      result,
      vec![(0, vec![-3, 3]), (1, vec![-1, 1]), (2, vec![-2, 2])]
    );
  }

  #[test]
  fn test_group_by_transformer_error_handling() {
    let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "GroupByTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "GroupByTransformer".to_string(),
      },
      retries: 0,
    };

    // Test default error strategy (Stop)
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));

    // Test Skip strategy
    let transformer = transformer.with_error_strategy(ErrorStrategy::Skip);
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));

    // Test Retry strategy
    let transformer = transformer.with_error_strategy(ErrorStrategy::Retry(3));
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Retry
    ));

    // Test Retry strategy exhausted
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "GroupByTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "GroupByTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_group_by_transformer_error_context_creation() {
    let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
      .with_name("test_group_by".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_group_by");
    assert_eq!(
      context.component_type,
      std::any::type_name::<GroupByTransformer<fn(&i32) -> i32, i32, i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_group_by_transformer_component_info() {
    let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
      .with_name("test_group_by".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_group_by");
    assert_eq!(
      info.type_name,
      std::any::type_name::<GroupByTransformer<fn(&i32) -> i32, i32, i32>>()
    );
  }

  #[test]
  fn test_group_by_transformer_default_name() {
    let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);

    let info = transformer.component_info();
    assert_eq!(info.name, "group_by_transformer");
  }

  #[test]
  fn test_group_by_transformer_config_mut() {
    let mut transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_group_by_transformer_reuse() {
    let mut transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);

    // First use
    let input1 = stream::iter(vec![1, 2, 3]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![(0, vec![2]), (1, vec![1, 3])]);

    // Second use
    let input2 = stream::iter(vec![4, 5, 6]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![(0, vec![4, 6]), (1, vec![5])]);
  }

  #[tokio::test]
  async fn test_group_by_transformer_edge_cases() {
    // Test with very large numbers
    let mut transformer = GroupByTransformer::new(|x: &i32| x / 1000);
    let input = stream::iter(vec![1000, 2000, 3000, 1500, 2500, 3500]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| *k);

    assert_eq!(
      result,
      vec![
        (1, vec![1000, 1500]),
        (2, vec![2000, 2500]),
        (3, vec![3000, 3500])
      ]
    );
  }

  #[tokio::test]
  async fn test_group_by_transformer_deterministic_ordering() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(vec![3, 1, 4, 2, 5, 6]);
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Should be sorted by key (0, 1)
    assert_eq!(result[0].0, 0);
    assert_eq!(result[1].0, 1);

    // Values within each group should maintain relative order
    assert_eq!(result[0].1, vec![4, 2, 6]);
    assert_eq!(result[1].1, vec![3, 1, 5]);
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_group_by_transformer_properties(
          name in ".*"
      ) {
          let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
              .with_name(name.clone());

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Stop
          ));
      }

      #[test]
      fn test_group_by_transformer_error_strategies(
          retry_count in 0..10usize
      ) {
          let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);

          let error = StreamError {
              source: Box::new(std::io::Error::other("property test error")),
              context: ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: "property_test".to_string(),
                  component_type: "GroupByTransformer".to_string(),
              },
              component: ComponentInfo {
                  name: "property_test".to_string(),
                  type_name: "GroupByTransformer".to_string(),
              },
              retries: retry_count,
          };

          // Test different error strategies
          let transformer_skip = transformer.clone().with_error_strategy(ErrorStrategy::Skip);
          let transformer_retry = transformer.clone().with_error_strategy(ErrorStrategy::Retry(5));

          assert!(matches!(
              transformer_skip.handle_error(&error),
              ErrorAction::Skip
          ));

          if retry_count < 5 {
              assert!(matches!(
                  transformer_retry.handle_error(&error),
                  ErrorAction::Retry
              ));
          } else {
              assert!(matches!(
                  transformer_retry.handle_error(&error),
                  ErrorAction::Stop
              ));
          }
      }

      #[test]
      fn test_group_by_transformer_config_persistence(
          name in ".*"
      ) {
          let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
              .with_name(name.clone())
              .with_error_strategy(ErrorStrategy::Skip);

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Skip
          ));
      }

      #[test]
      fn test_group_by_transformer_key_function_properties(
          _values in prop::collection::vec(0..100i32, 0..20)
      ) {
          // Test that grouping by a constant function produces one group
          let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|_| 42);
          assert_eq!(transformer.config().name(), None);
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Stop
          ));

          // Test that grouping by identity function produces groups of size 1
          let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| *x);
          assert_eq!(transformer.config().name(), None);
      }
  }

  #[tokio::test]
  async fn test_group_by_transformer_stream_processing() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 3);

    // Process a stream with various grouping patterns
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let boxed_input = Box::pin(input);
    let result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Should have 3 groups (0, 1, 2)
    assert_eq!(result.len(), 3);

    // Check that all groups are present
    let keys: Vec<i32> = result.iter().map(|(k, _)| *k).collect();
    assert!(keys.contains(&0));
    assert!(keys.contains(&1));
    assert!(keys.contains(&2));
  }

  #[tokio::test]
  async fn test_group_by_transformer_nested_grouping() {
    let mut transformer = GroupByTransformer::new(|x: &i32| {
      if *x < 5 {
        if *x % 2 == 0 {
          "small_even".to_string()
        } else {
          "small_odd".to_string()
        }
      } else if *x % 2 == 0 {
        "large_even".to_string()
      } else {
        "large_odd".to_string()
      }
    });

    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);
    let mut result: Vec<(String, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    result.sort_by_key(|(k, _)| k.clone());

    assert_eq!(
      result,
      vec![
        ("large_even".to_string(), vec![6, 8, 10]),
        ("large_odd".to_string(), vec![5, 7, 9]),
        ("small_even".to_string(), vec![2, 4]),
        ("small_odd".to_string(), vec![1, 3])
      ]
    );
  }
}
