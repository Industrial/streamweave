use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;
pub struct FlatMapOperator<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: Send + Sync + 'static,
  O: Send + Sync + 'static,
{
  f: F,
  _phantom_i: std::marker::PhantomData<I>,
  _phantom_o: std::marker::PhantomData<O>,
}

impl<F, I, O> FlatMapOperator<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: Send + Sync + 'static,
  O: Send + Sync + 'static,
{
  pub fn new(f: F) -> Self {
    Self {
      f,
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }
}

impl<F, I, O, E> EffectStreamOperator<I, E, O> for FlatMapOperator<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: Send + Sync + 'static,
  O: Send + Sync + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<O, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<I, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let f = self.f.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<O, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(item)) = stream_clone.next().await {
          let mapped_items = f(item);
          for mapped_item in mapped_items {
            new_stream_clone.push(mapped_item).await.unwrap();
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
  async fn test_flat_map_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = FlatMapOperator::new(|x: i32| vec![x * 2, x * 3]);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![2, 3, 4, 6, 6, 9]);
  }

  #[tokio::test]
  async fn test_flat_map_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = FlatMapOperator::new(|_: i32| Vec::<i32>::new());
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_flat_map_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = FlatMapOperator::new(|x: i32| vec![x * 2, x * 3]);
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
    final_results.sort();
    assert_eq!(*final_results, vec![2, 2, 3, 3, 4, 4, 6, 6, 6, 6, 9, 9]);
  }
}
