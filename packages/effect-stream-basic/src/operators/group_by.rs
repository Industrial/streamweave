use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use tokio;

pub struct GroupByOperator<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: Send + Sync + 'static,
  K: Send + Sync + Hash + Eq + Ord + 'static,
{
  key_fn: F,
  _phantom_t: std::marker::PhantomData<T>,
  _phantom_k: std::marker::PhantomData<K>,
}

impl<F, T, K> GroupByOperator<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: Send + Sync + 'static,
  K: Send + Sync + Hash + Eq + Ord + 'static,
{
  pub fn new(key_fn: F) -> Self {
    Self {
      key_fn,
      _phantom_t: std::marker::PhantomData,
      _phantom_k: std::marker::PhantomData,
    }
  }
}

impl<F, T, K, E> EffectStreamOperator<T, E, (K, Vec<T>)> for GroupByOperator<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: Send + Sync + Clone + 'static,
  K: Send + Sync + Clone + Hash + Eq + Ord + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future =
    Pin<Box<dyn Future<Output = EffectResult<EffectStream<(K, Vec<T>), E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let key_fn = self.key_fn.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<(K, Vec<T>), E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut groups = HashMap::new();
        while let Ok(Some(item)) = stream_clone.next().await {
          let key = key_fn(&item);
          groups.entry(key).or_insert_with(Vec::new).push(item);
        }

        // Sort groups by key for deterministic output
        let mut groups = groups.into_iter().collect::<Vec<_>>();
        groups.sort_by_key(|(k, _)| k.clone());

        for group in groups {
          new_stream_clone.push(group).await.unwrap();
        }
        new_stream_clone.close().await.unwrap();
      });

      Ok(new_stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;
  use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
  };

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_group_by_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=6 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = GroupByOperator::new(|x: &i32| x % 2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }
    results.sort_by_key(|(k, _)| *k);

    assert_eq!(results, vec![(0, vec![2, 4, 6]), (1, vec![1, 3, 5])]);
  }

  #[tokio::test]
  async fn test_group_by_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = GroupByOperator::new(|x: &i32| x % 2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::new());
  }

  #[tokio::test]
  async fn test_group_by_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=6 {
        stream_clone.push(i).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = GroupByOperator::new(|x: &i32| x % 2);
    let new_stream = operator.transform(stream).await.unwrap();
    let new_stream_clone = new_stream.clone();

    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone1 = results.clone();
    let results_clone2 = results.clone();

    let handle1 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream.next().await {
        results_clone1.lock().await.push(value);
      }
    });

    let handle2 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream_clone.next().await {
        results_clone2.lock().await.push(value);
      }
    });

    handle1.await.unwrap();
    handle2.await.unwrap();

    let mut final_results = results.lock().await;
    final_results.sort_by_key(|(k, _)| *k);
    assert_eq!(final_results.len(), 2);
    assert!(final_results.contains(&(0, vec![2, 4, 6])));
    assert!(final_results.contains(&(1, vec![1, 3, 5])));
  }
}
