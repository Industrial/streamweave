use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct WindowOperator<T>
where
  T: Send + Sync + 'static,
{
  size: usize,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> WindowOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(size: usize) -> Result<Self, String> {
    if size == 0 {
      return Err("Window size must be greater than zero".to_string());
    }
    Ok(Self {
      size,
      _phantom: std::marker::PhantomData,
    })
  }
}

impl<T, E> EffectStreamOperator<T, E, Vec<T>> for WindowOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + 'static + std::fmt::Debug,
{
  type Future =
    Pin<Box<dyn Future<Output = EffectResult<EffectStream<Vec<T>, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let size = self.size;

    Box::pin(async move {
      let new_stream = EffectStream::<Vec<T>, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut window: VecDeque<T> = VecDeque::with_capacity(size);

        while let Ok(Some(item)) = stream_clone.next().await {
          window.push_back(item);

          if window.len() == size {
            new_stream_clone
              .push(window.iter().cloned().collect::<Vec<_>>())
              .await
              .unwrap();
            window.clear(); // Clear the entire window instead of just popping one item
          }
        }

        // Emit any remaining items as a partial window
        if !window.is_empty() {
          new_stream_clone
            .push(window.iter().cloned().collect::<Vec<_>>())
            .await
            .unwrap();
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
  async fn test_window_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=7 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = WindowOperator::new(3).unwrap();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![vec![1, 2, 3], vec![4, 5, 6], vec![7]]);
  }

  #[tokio::test]
  async fn test_window_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = WindowOperator::new(3).unwrap();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_window_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=7 {
        stream_clone.push(i).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = WindowOperator::new(3).unwrap();
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
    assert_eq!(final_results.len(), 3);
    assert!(final_results.contains(&vec![1, 2, 3]));
    assert!(final_results.contains(&vec![4, 5, 6]));
    assert!(final_results.contains(&vec![7]));
  }
}
