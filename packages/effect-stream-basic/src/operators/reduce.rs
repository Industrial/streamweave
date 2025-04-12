use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct ReduceOperator<T, Acc, F>
where
  T: Send + Sync + 'static,
  Acc: Send + Sync + Clone + 'static,
  F: Fn(Acc, T) -> Acc + Send + Clone + 'static,
{
  accumulator: Acc,
  reducer: F,
  _phantom: std::marker::PhantomData<T>,
}

impl<T, Acc, F> ReduceOperator<T, Acc, F>
where
  T: Send + Sync + 'static,
  Acc: Send + Sync + Clone + 'static,
  F: Fn(Acc, T) -> Acc + Send + Clone + 'static,
{
  pub fn new(initial: Acc, reducer: F) -> Self {
    Self {
      accumulator: initial,
      reducer,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, Acc, F, E> EffectStreamOperator<T, E, Acc> for ReduceOperator<T, Acc, F>
where
  T: Send + Sync + 'static,
  Acc: Send + Sync + Clone + 'static,
  F: Fn(Acc, T) -> Acc + Send + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future =
    Pin<Box<dyn Future<Output = EffectResult<EffectStream<Acc, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let reducer = self.reducer.clone();
    let initial = self.accumulator.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<Acc, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut acc = initial;
        while let Ok(Some(item)) = stream_clone.next().await {
          acc = reducer(acc.clone(), item);
          new_stream_clone.push(acc.clone()).await.unwrap();
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
  async fn test_reduce_operator_sum() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = ReduceOperator::new(0, |acc, x| acc + x);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 3, 6, 10, 15]);
  }

  #[tokio::test]
  async fn test_reduce_operator_string_concat() {
    let stream = EffectStream::<String, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for s in vec!["a", "b", "c"] {
        stream_clone.push(s.to_string()).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = ReduceOperator::new(String::new(), |mut acc: String, x: String| {
      acc.push_str(&x);
      acc
    });
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(
      results,
      vec!["a".to_string(), "ab".to_string(), "abc".to_string()]
    );
  }

  #[tokio::test]
  async fn test_reduce_operator_with_error() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in vec![2, 3, 4, 5, 6] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = ReduceOperator::new(0, |acc, x| if x % 2 == 0 { acc + x } else { acc });
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![2, 2, 6, 6, 12]);
  }

  #[tokio::test]
  async fn test_reduce_operator_custom_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Counter {
      count: i32,
      sum: i32,
    }

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = ReduceOperator::new(Counter { count: 0, sum: 0 }, |acc, x| Counter {
      count: acc.count + 1,
      sum: acc.sum + x,
    });
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(
      results,
      vec![
        Counter { count: 1, sum: 1 },
        Counter { count: 2, sum: 3 },
        Counter { count: 3, sum: 6 },
        Counter { count: 4, sum: 10 },
        Counter { count: 5, sum: 15 },
      ]
    );
  }

  #[tokio::test]
  async fn test_reduce_operator_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = ReduceOperator::new(0, |acc, x| acc + x);
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
    assert_eq!(final_results.len(), 5);
    assert!(final_results.contains(&1));
    assert!(final_results.contains(&3));
    assert!(final_results.contains(&6));
    assert!(final_results.contains(&10));
    assert!(final_results.contains(&15));
  }
}
