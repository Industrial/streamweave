use effect_stream::{EffectResult, EffectStream, EffectStreamSink};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

struct ArraySinkState<T, const N: usize> {
  array: [Option<T>; N],
  index: usize,
  sender: Option<oneshot::Sender<[Option<T>; N]>>,
}

/// A sink that collects values from a stream into an array
pub struct ArrayStreamSink<T, const N: usize>
where
  T: Send + Sync + Clone + 'static,
{
  state: Arc<Mutex<ArraySinkState<T, N>>>,
  receiver: oneshot::Receiver<[Option<T>; N]>,
}

impl<T, const N: usize> ArrayStreamSink<T, N>
where
  T: Send + Sync + Clone + 'static,
{
  /// Create a new ArrayStreamSink
  pub fn new() -> Self {
    let (sender, receiver) = oneshot::channel();
    Self {
      state: Arc::new(Mutex::new(ArraySinkState {
        array: std::array::from_fn(|_| None),
        index: 0,
        sender: Some(sender),
      })),
      receiver,
    }
  }

  /// Get the collected array
  pub async fn into_array(self) -> Option<[T; N]> {
    let array = self.receiver.await.ok()?;
    let mut arr = std::mem::MaybeUninit::<[T; N]>::uninit();
    let ptr = arr.as_mut_ptr() as *mut T;
    unsafe {
      for (i, value) in array.into_iter().enumerate() {
        value.as_ref()?;
        ptr.add(i).write(value.unwrap());
      }
      Some(arr.assume_init())
    }
  }
}

impl<T, const N: usize> Default for ArrayStreamSink<T, N>
where
  T: Send + Sync + Clone + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, E, const N: usize> EffectStreamSink<T, E> for ArrayStreamSink<T, N>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<(), E>> + Send + 'static>>;

  fn consume(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let state = self.state.clone();

    Box::pin(async move {
      let stream = stream_clone;
      while let Ok(Some(value)) = stream.next().await {
        let mut guard = state.lock().await;
        let index = guard.index;
        if index >= N {
          return Err(effect_stream::EffectError::Processing(
            "Array is full".to_string(),
          ));
        }
        guard.array[index] = Some(value);
        guard.index += 1;
        if guard.index == N {
          if let Some(sender) = guard.sender.take() {
            let _ = sender.send(guard.array.clone());
          }
        }
        drop(guard);
      }

      // Send the array when the stream ends
      let mut guard = state.lock().await;
      if let Some(sender) = guard.sender.take() {
        let _ = sender.send(guard.array.clone());
      }
      drop(guard);

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
  async fn test_array_stream_sink_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let sink = ArrayStreamSink::<i32, 3>::new();
    sink.consume(stream).await.unwrap();
    let array = sink.into_array().await.unwrap();
    assert_eq!(array, [1, 2, 3]);
  }

  #[tokio::test]
  async fn test_array_stream_sink_overflow() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.push(4).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let sink = ArrayStreamSink::<i32, 3>::new();
    let result = sink.consume(stream).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_array_stream_sink_underflow() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let sink = ArrayStreamSink::<i32, 3>::new();
    sink.consume(stream).await.unwrap();
    assert!(sink.into_array().await.is_none());
  }
}
