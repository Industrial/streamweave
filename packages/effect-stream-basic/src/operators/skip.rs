use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct SkipOperator<T>
where
  T: Send + Sync + 'static,
{
  skip: usize,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> SkipOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(skip: usize) -> Self {
    Self {
      skip,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for SkipOperator<T>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let skip = self.skip;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut count = 0;
        while let Ok(Some(item)) = stream_clone.next().await {
          if count >= skip {
            new_stream_clone.push(item).await.unwrap();
          }
          count += 1;
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
  use tokio::{sync::Mutex, time::Duration};

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_skip_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = SkipOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![3, 4, 5]);
  }

  #[tokio::test]
  async fn test_skip_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = SkipOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_skip_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = SkipOperator::new(2);
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
    assert_eq!(final_results.len(), 6); // Each value is received twice due to cloning
    assert!(final_results.contains(&3));
    assert!(final_results.contains(&4));
    assert!(final_results.contains(&5));
  }
}
