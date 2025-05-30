use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct SplitOperator<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: Send + Sync + 'static,
{
  predicate: F,
  _phantom: std::marker::PhantomData<T>,
}

impl<F, T> SplitOperator<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: Send + Sync + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<F, T, E> EffectStreamOperator<T, E, Vec<T>> for SplitOperator<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static + std::fmt::Debug,
{
  type Future =
    Pin<Box<dyn Future<Output = EffectResult<EffectStream<Vec<T>, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let predicate = self.predicate.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<Vec<T>, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut current_chunk = Vec::new();
        let mut chunks = Vec::new();

        while let Ok(Some(item)) = stream_clone.next().await {
          if predicate(&item) && !current_chunk.is_empty() {
            chunks.push(std::mem::take(&mut current_chunk));
          }
          current_chunk.push(item);
        }

        if !current_chunk.is_empty() {
          chunks.push(current_chunk);
        }

        for chunk in chunks {
          new_stream_clone.push(chunk).await.unwrap();
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
  async fn test_split_by_even_numbers() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in [1, 2, 3, 4, 5, 6] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = SplitOperator::new(|x: &i32| x % 2 == 0);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![vec![1], vec![2, 3], vec![4, 5], vec![6]]);
  }

  #[tokio::test]
  async fn test_split_no_splits() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = SplitOperator::new(|x: &i32| *x < 0); // No negatives in input
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![vec![1, 2, 3, 4, 5]]);
  }

  #[tokio::test]
  async fn test_split_strings() {
    let stream = EffectStream::<String, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for s in ["hello", "", "world", "", "rust"] {
        stream_clone.push(s.to_string()).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = SplitOperator::new(|s: &String| s.is_empty());
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(
      results,
      vec![
        vec!["hello".to_string()],
        vec!["".to_string(), "world".to_string()],
        vec!["".to_string(), "rust".to_string()],
      ]
    );
  }

  #[tokio::test]
  async fn test_split_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in [1, 2, 3, 4, 5, 6] {
        stream_clone.push(i).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = SplitOperator::new(|x: &i32| x % 2 == 0);
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

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 4);
    assert!(final_results.contains(&vec![1]));
    assert!(final_results.contains(&vec![2, 3]));
    assert!(final_results.contains(&vec![4, 5]));
    assert!(final_results.contains(&vec![6]));
  }
}
