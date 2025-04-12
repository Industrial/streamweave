use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::collections::HashSet;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;

pub struct DedupeOperator<T>
where
  T: Send + Sync + Hash + Eq + 'static,
{
  seen: HashSet<T>,
}

impl<T> DedupeOperator<T>
where
  T: Send + Sync + Hash + Eq + 'static,
{
  pub fn new() -> Self {
    Self {
      seen: HashSet::new(),
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for DedupeOperator<T>
where
  T: Send + Sync + Hash + Eq + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let mut seen = self.seen.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(item)) = stream_clone.next().await {
          if seen.insert(item.clone()) {
            new_stream_clone.push(item).await.unwrap();
          }
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
  async fn test_dedupe_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in vec![1, 2, 2, 3, 3, 3, 4] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DedupeOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3, 4]);
  }

  #[tokio::test]
  async fn test_dedupe_strings() {
    let stream = EffectStream::<String, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for s in vec!["a", "b", "b", "c", "a"] {
        stream_clone.push(s.to_string()).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DedupeOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec!["a", "b", "c"]);
  }

  #[tokio::test]
  async fn test_dedupe_empty() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = DedupeOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_dedupe_all_duplicates() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in vec![1, 1, 1, 1] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DedupeOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1]);
  }

  #[tokio::test]
  async fn test_dedupe_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in vec![1, 2, 2, 3, 3, 3, 4] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DedupeOperator::new();
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

    assert_eq!(results1, vec![1, 2, 3, 4]);
    assert_eq!(results2, vec![1, 2, 3, 4]);
  }
}
