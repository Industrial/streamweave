use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use futures::{Stream, StreamExt};
use std::collections::HashSet;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::Mutex;

pub struct DistinctOperator<T>
where
  T: Send + Sync + Hash + Eq + 'static,
{
  seen: Arc<Mutex<HashSet<T>>>,
}

impl<T> DistinctOperator<T>
where
  T: Send + Sync + Hash + Eq + 'static,
{
  pub fn new() -> Self {
    Self {
      seen: Arc::new(Mutex::new(HashSet::new())),
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for DistinctOperator<T>
where
  T: Send + Sync + Hash + Eq + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let new_stream = EffectStream::new();
    let new_stream_clone = new_stream.clone();
    let seen = Arc::clone(&self.seen);

    Box::pin(async move {
      let stream_clone = stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(item)) = stream_clone.next().await {
          let item_clone = item.clone();
          let seen_clone = Arc::clone(&seen);
          let new_stream_clone = new_stream_clone.clone();

          // Spawn a separate task for the write operation
          tokio::spawn(async move {
            let mut seen = seen_clone.lock().await;
            if seen.insert(item_clone.clone()) {
              new_stream_clone.push(item_clone).await.unwrap();
            }
          });
        }

        new_stream_clone.close().await.unwrap();
      });

      Ok(new_stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_distinct_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in vec![1, 2, 2, 3, 3, 3] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DistinctOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_distinct_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = DistinctOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_distinct_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in vec![1, 2, 2, 3, 3, 3] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DistinctOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results1 = Vec::new();
    let mut results2 = Vec::new();
    let mut new_stream_clone1 = new_stream.clone();
    let mut new_stream_clone2 = new_stream.clone();

    let handle1 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream_clone1.next().await {
        results1.push(value);
      }
      results1
    });

    let handle2 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream_clone2.next().await {
        results2.push(value);
      }
      results2
    });

    let (results1, results2) = tokio::join!(handle1, handle2);
    let results1 = results1.unwrap();
    let results2 = results2.unwrap();

    assert_eq!(results1, vec![1, 2, 3]);
    assert_eq!(results2, vec![1, 2, 3]);
  }
}
