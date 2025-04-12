use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct MapOperator<F, T, B>
where
  F: Fn(T) -> B + Send + Clone + 'static,
  T: Send + Sync + 'static,
  B: Send + Sync + 'static,
{
  f: F,
  _phantom_t: std::marker::PhantomData<T>,
  _phantom_b: std::marker::PhantomData<B>,
}

impl<F, T, B> MapOperator<F, T, B>
where
  F: Fn(T) -> B + Send + Clone + 'static,
  T: Send + Sync + 'static,
  B: Send + Sync + 'static,
{
  pub fn new(f: F) -> Self {
    Self {
      f,
      _phantom_t: std::marker::PhantomData,
      _phantom_b: std::marker::PhantomData,
    }
  }
}

impl<F, T, B, E> EffectStreamOperator<T, E, B> for MapOperator<F, T, B>
where
  F: Fn(T) -> B + Send + Clone + 'static,
  T: Send + Sync + Clone + 'static,
  B: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<B, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let f = self.f.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<B, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(value)) = stream_clone.next().await {
          new_stream_clone.push(f(value)).await.unwrap();
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
  async fn test_map_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = MapOperator::new(|x| x * 2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_map_type_conversion() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = MapOperator::new(|x: i32| x.to_string());
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec!["1", "2", "3"]);
  }

  #[tokio::test]
  async fn test_map_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = MapOperator::new(|x| x * 2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_map_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = MapOperator::new(|x| x * 2);
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
    assert_eq!(final_results.len(), 3); // Each value is consumed exactly once
    assert!(final_results.contains(&2));
    assert!(final_results.contains(&4));
    assert!(final_results.contains(&6));
  }
}
