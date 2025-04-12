use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use futures::{Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;

pub struct FlattenOperator<T>
where
  T: Send + Sync + 'static,
{
  _phantom: std::marker::PhantomData<T>,
}

impl<T> FlattenOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<Vec<T>, E, T> for FlattenOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<Vec<T>, E>) -> Self::Future {
    let stream_clone = stream.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let mut new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(items)) = stream_clone.next().await {
          for item in items {
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
  async fn test_flatten_basic() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![1, 2, 3]).await.unwrap();
      stream_clone.push(vec![4, 5]).await.unwrap();
      stream_clone.push(vec![6]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = FlattenOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
  }

  #[tokio::test]
  async fn test_flatten_empty_input() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = FlattenOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_flatten_mixed_input() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.push(vec![1]).await.unwrap();
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.push(vec![2, 3]).await.unwrap();
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = FlattenOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_flatten_concurrent() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![1, 2]).await.unwrap();
      stream_clone.push(vec![3, 4]).await.unwrap();
      stream_clone.push(vec![5]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = FlattenOperator::new();
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

    assert_eq!(results1, vec![1, 2, 3, 4, 5]);
    assert_eq!(results2, vec![1, 2, 3, 4, 5]);
  }
}
