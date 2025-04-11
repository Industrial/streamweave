use effect_stream::{EffectResult, EffectStream, EffectStreamSink};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A sink that collects values into a vector
pub struct VecStreamSink<T>
where
  T: Send + Sync + Clone + 'static,
{
  vec: Arc<Mutex<Vec<T>>>,
}

impl<T> VecStreamSink<T>
where
  T: Send + Sync + Clone + 'static,
{
  /// Create a new sink
  pub fn new() -> Self {
    Self {
      vec: Arc::new(Mutex::new(Vec::new())),
    }
  }

  /// Get the collected values
  pub async fn into_vec(self) -> Vec<T> {
    let vec = self.vec.lock().await;
    vec.clone()
  }
}

impl<T> Default for VecStreamSink<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, E> EffectStreamSink<T, E> for VecStreamSink<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<(), E>> + Send + 'static>>;

  fn consume(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let vec = self.vec.clone();

    Box::pin(async move {
      let stream = stream_clone;
      while let Ok(Some(value)) = stream.next().await {
        let mut vec = vec.lock().await;
        vec.push(value);
      }
      Ok(())
    })
  }
}

#[cfg(test)]
mod tests {
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
  async fn test_vec_stream_sink_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let sink = VecStreamSink::<i32>::new();
    sink.consume(stream).await.unwrap();
    let vec = sink.into_vec().await;
    assert_eq!(vec, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_vec_stream_sink_empty() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let sink = VecStreamSink::<i32>::new();
    sink.consume(stream).await.unwrap();
    let vec = sink.into_vec().await;
    assert_eq!(vec, Vec::<i32>::new());
  }
}
