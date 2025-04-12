use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use futures::{Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct InterleaveOperator<T>
where
  T: Send + Sync + 'static,
{
  other: Arc<Mutex<Pin<Box<dyn Stream<Item = T> + Send>>>>,
}

impl<T> InterleaveOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(other: Pin<Box<dyn Stream<Item = T> + Send>>) -> Self {
    Self {
      other: Arc::new(Mutex::new(other)),
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for InterleaveOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(
    &self,
    stream: EffectStream<T, E>,
  ) -> Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>> {
    let new_stream = EffectStream::new();
    let new_stream_clone = new_stream.clone();
    let other = Arc::clone(&self.other);

    Box::pin(async move {
      let stream_clone = stream.clone();

      tokio::spawn(async move {
        let stream_clone = stream_clone;
        let mut other_clone = other.lock().await;
        let mut first = true;

        while let (Ok(Some(item1)), Some(item2)) =
          (stream_clone.next().await, other_clone.next().await)
        {
          if first {
            new_stream_clone.push(item1).await.unwrap();
            new_stream_clone.push(item2).await.unwrap();
          } else {
            new_stream_clone.push(item2).await.unwrap();
            new_stream_clone.push(item1).await.unwrap();
          }
          first = !first;
        }

        // Push remaining items from both streams
        while let Ok(Some(item)) = stream_clone.next().await {
          new_stream_clone.push(item).await.unwrap();
        }

        while let Some(item) = other_clone.next().await {
          new_stream_clone.push(item).await.unwrap();
        }

        new_stream_clone.close().await.unwrap();
      });

      Ok(new_stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use futures::stream;

  use super::*;

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_interleave_basic() {
    let other = stream::iter(vec![4, 5, 6].into_iter());
    let operator = InterleaveOperator::new(Box::pin(other));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 4, 2, 5, 3, 6]);
  }

  #[tokio::test]
  async fn test_interleave_empty_input() {
    let other = stream::iter(Vec::<i32>::new());
    let operator = InterleaveOperator::new(Box::pin(other));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_interleave_concurrent() {
    let other = stream::iter(vec![4, 5, 6].into_iter());
    let operator = InterleaveOperator::new(Box::pin(other));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results1 = Vec::new();
    let mut results2 = Vec::new();
    let new_stream_clone1 = new_stream.clone();
    let new_stream_clone2 = new_stream.clone();

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

    assert_eq!(results1, vec![1, 4, 2, 5, 3, 6]);
    assert_eq!(results2, vec![1, 4, 2, 5, 3, 6]);
  }

  #[tokio::test]
  async fn test_interleave_uneven() {
    let other = stream::iter(vec![4, 5].into_iter());
    let operator = InterleaveOperator::new(Box::pin(other));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 4, 2, 5, 3]);
  }
}
