use effect_core::{Functor, Monad};
use futures::{Stream, StreamExt};
use std::future::Future;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tokio::sync::Notify;

use crate::error::{EffectError, EffectResult};

/// A stream of values that are produced by effects
pub struct EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  inner: Arc<Mutex<InnerStream<T, E>>>,
}

struct InnerStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  values: Vec<T>,
  is_closed: bool,
  error: Option<EffectError<E>>,
  notify: Arc<Notify>,
}

/// Type alias for a locked inner stream
type LockedStream<T, E> = Arc<Mutex<InnerStream<T, E>>>;

/// Type alias for the state tuple used in stream processing
type StreamState<T, E, B, F> = (
  LockedStream<T, E>,
  Arc<Mutex<F>>,
  Option<LockedStream<B, E>>,
  bool,
);

impl<T, E> EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  /// Create a new empty stream
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(InnerStream::new())),
    }
  }

  /// Create a stream from a futures Stream
  pub fn from_stream<S>(stream: S) -> Self
  where
    S: Stream<Item = EffectResult<T, E>> + Send + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    let inner = Arc::new(Mutex::new(InnerStream::new()));

    let inner_clone = Arc::clone(&inner);
    tokio::spawn(async move {
      let mut stream = Box::pin(stream);
      while let Some(result) = stream.next().await {
        let mut guard = inner_clone.lock().await;
        match result {
          Ok(value) => guard.values.push(value),
          Err(err) => {
            guard.error = Some(err);
            break;
          }
        }
      }
      let mut guard = inner_clone.lock().await;
      guard.is_closed = true;
    });

    Self { inner }
  }

  /// Push a value to the stream
  pub async fn push(&self, value: T) -> EffectResult<(), E> {
    let mut stream = self.inner.lock().await;
    if stream.is_closed {
      return Err(EffectError::Closed);
    }
    stream.values.push(value);
    stream.notify.notify_one();
    Ok(())
  }

  /// Close the stream
  pub async fn close(&self) -> EffectResult<(), E> {
    let mut stream = self.inner.lock().await;
    if stream.is_closed {
      return Err(EffectError::Closed);
    }
    stream.is_closed = true;
    stream.notify.notify_one();
    Ok(())
  }

  /// Check if the stream is closed
  pub async fn is_closed(&self) -> bool {
    let inner = self.inner.lock().await;
    inner.is_closed
  }

  /// Get the next value from the stream
  pub async fn next(&self) -> EffectResult<Option<T>, E> {
    let mut stream = self.inner.lock().await;
    if let Some(err) = &stream.error {
      return Err(err.clone());
    }
    if stream.values.is_empty() {
      if stream.is_closed {
        return Ok(None);
      }
      drop(stream);
      let notify = self.inner.lock().await.notify.clone();
      notify.notified().await;
      stream = self.inner.lock().await;
      if let Some(err) = &stream.error {
        return Err(err.clone());
      }
      if stream.values.is_empty() && stream.is_closed {
        return Ok(None);
      }
    }
    Ok(Some(stream.values.remove(0)))
  }

  /// Set an error on the stream
  pub async fn set_error(&self, error: E) -> EffectResult<(), E> {
    let mut stream = self.inner.lock().await;
    stream.error = Some(EffectError::Custom(error));
    stream.notify.notify_one();
    Ok(())
  }

  pub async fn map<F, B>(self, f: F) -> EffectStream<B, E>
  where
    F: Fn(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
    E: Clone,
  {
    let inner = Arc::new(Mutex::new(InnerStream::new()));
    let source_inner = self.inner;
    let f = Arc::new(f);
    let inner_clone = inner.clone();

    tokio::spawn(async move {
      loop {
        let mut source = source_inner.lock().await;
        if let Some(err) = &source.error {
          let mut target = inner_clone.lock().await;
          target.error = Some(err.clone());
          target.notify.notify_one();
          break;
        }
        if source.values.is_empty() {
          if source.is_closed {
            let mut target = inner_clone.lock().await;
            target.is_closed = true;
            target.notify.notify_one();
            break;
          }
          drop(source);
          let notify = source_inner.lock().await.notify.clone();
          notify.notified().await;
          continue;
        }
        let value = source.values.remove(0);
        drop(source);
        let result = (*f)(value);
        let mut target = inner_clone.lock().await;
        target.values.push(result);
        target.notify.notify_one();
      }
    });

    EffectStream { inner }
  }

  pub async fn bind<F, B>(self, f: F) -> EffectStream<B, E>
  where
    F: Fn(T) -> EffectStream<B, E> + Send + Sync + 'static,
    B: Send + Sync + 'static,
    E: Clone,
  {
    let inner = Arc::new(Mutex::new(InnerStream::new()));
    let source_inner = self.inner;
    let f = Arc::new(f);
    let inner_clone = inner.clone();

    tokio::spawn(async move {
      loop {
        let mut source = source_inner.lock().await;
        if let Some(err) = &source.error {
          let mut target = inner_clone.lock().await;
          target.error = Some(err.clone());
          target.notify.notify_one();
          break;
        }
        if source.values.is_empty() {
          if source.is_closed {
            let mut target = inner_clone.lock().await;
            target.is_closed = true;
            target.notify.notify_one();
            break;
          }
          drop(source);
          let notify = source_inner.lock().await.notify.clone();
          notify.notified().await;
          continue;
        }
        let value = source.values.remove(0);
        drop(source);
        let mut stream = (*f)(value);
        loop {
          match stream.next().await {
            Ok(Some(result)) => {
              let mut target = inner_clone.lock().await;
              target.values.push(result);
              target.notify.notify_one();
            }
            Ok(None) => break,
            Err(err) => {
              let mut target = inner_clone.lock().await;
              target.error = Some(err);
              target.notify.notify_one();
              break;
            }
          }
        }
      }
    });

    EffectStream { inner }
  }
}

impl<T, E> Stream for EffectStream<T, E>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + 'static,
{
  type Item = EffectResult<T, E>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let inner = self.inner.clone();
    let mut guard = futures::executor::block_on(inner.lock());

    if let Some(err) = &guard.error {
      Poll::Ready(Some(Err(err.clone())))
    } else if !guard.values.is_empty() {
      Poll::Ready(Some(Ok(guard.values.remove(0))))
    } else if guard.is_closed {
      Poll::Ready(None)
    } else {
      cx.waker().wake_by_ref();
      Poll::Pending
    }
  }
}

impl<T, E> Clone for EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T, E> Default for EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, E> FromIterator<T> for EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let values: Vec<T> = iter.into_iter().collect();
    Self {
      inner: Arc::new(Mutex::new(InnerStream {
        values,
        error: None,
        is_closed: false,
        notify: Arc::new(Notify::new()),
      })),
    }
  }
}

impl<T, E> Monad<T> for EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  type HigherSelf<U: Send + Sync + 'static> = EffectStream<U, E>;

  fn pure(a: T) -> Self::HigherSelf<T> {
    EffectStream::from_iter(std::iter::once(a))
  }

  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    let f = Arc::new(Mutex::new(f));
    EffectStream::from_stream(Box::pin(futures::stream::unfold(
      (self.inner, f, None::<Arc<Mutex<InnerStream<B, E>>>>, false),
      move |state| process_next_value(state),
    )))
  }
}

/// Helper function to process the next value from a stream
async fn process_next_value<T, E, B, F>(
  state: StreamState<T, E, B, F>,
) -> Option<(EffectResult<B, E>, StreamState<T, E, B, F>)>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
  B: Send + Sync + 'static,
  F: FnMut(T) -> EffectStream<B, E> + Send + Sync + 'static,
{
  let (input_stream, f, current_stream, input_exhausted) = state;

  if input_exhausted && current_stream.is_none() {
    return None;
  }

  if let Some(stream) = current_stream {
    process_current_stream(stream, input_stream, f, input_exhausted).await
  } else {
    process_input_stream(input_stream, f).await
  }
}

/// Helper function to process the current stream
async fn process_current_stream<T, E, B, F>(
  stream: LockedStream<B, E>,
  input_stream: LockedStream<T, E>,
  f: Arc<Mutex<F>>,
  input_exhausted: bool,
) -> Option<(EffectResult<B, E>, StreamState<T, E, B, F>)>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
  B: Send + Sync + 'static,
  F: FnMut(T) -> EffectStream<B, E> + Send + Sync + 'static,
{
  let next = get_next_value(&stream).await;
  match next {
    Some(result) => Some((result, (input_stream, f, Some(stream), input_exhausted))),
    None => process_next_input(input_stream, f).await,
  }
}

/// Helper function to process the input stream
async fn process_input_stream<T, E, B, F>(
  input_stream: LockedStream<T, E>,
  f: Arc<Mutex<F>>,
) -> Option<(EffectResult<B, E>, StreamState<T, E, B, F>)>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
  B: Send + Sync + 'static,
  F: FnMut(T) -> EffectStream<B, E> + Send + Sync + 'static,
{
  let next_input = get_next_value(&input_stream).await;
  match next_input {
    Some(Ok(value)) => {
      let f_clone = f.clone();
      let mut f_guard = f_clone.lock().await;
      let next_stream = f_guard(value);
      let next = get_next_value(&next_stream.inner).await;
      match next {
        Some(result) => Some((result, (input_stream, f, Some(next_stream.inner), false))),
        None => None,
      }
    }
    Some(Err(e)) => Some((Err(e), (input_stream, f, None, true))),
    None => None,
  }
}

/// Helper function to process the next input value
async fn process_next_input<T, E, B, F>(
  input_stream: LockedStream<T, E>,
  f: Arc<Mutex<F>>,
) -> Option<(EffectResult<B, E>, StreamState<T, E, B, F>)>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
  B: Send + Sync + 'static,
  F: FnMut(T) -> EffectStream<B, E> + Send + Sync + 'static,
{
  let next_input = get_next_value(&input_stream).await;
  match next_input {
    Some(Ok(value)) => {
      let f_clone = f.clone();
      let mut f_guard = f_clone.lock().await;
      let next_stream = f_guard(value);
      let next = get_next_value(&next_stream.inner).await;
      match next {
        Some(result) => Some((result, (input_stream, f, Some(next_stream.inner), false))),
        None => None,
      }
    }
    Some(Err(e)) => Some((Err(e), (input_stream, f, None, true))),
    None => None,
  }
}

/// Helper function to get the next value from a stream
async fn get_next_value<T, E>(stream: &Arc<Mutex<InnerStream<T, E>>>) -> Option<EffectResult<T, E>>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  loop {
    let mut stream_guard = stream.lock().await;
    if let Some(err) = stream_guard.error.clone() {
      return Some(Err(err));
    }
    if !stream_guard.values.is_empty() {
      return Some(Ok(stream_guard.values.remove(0)));
    }
    if stream_guard.is_closed {
      return None;
    }
    let notify = stream_guard.notify.clone();
    drop(stream_guard);
    notify.notified().await;
  }
}

impl<T, E> InnerStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  fn new() -> Self {
    Self {
      values: Vec::new(),
      is_closed: false,
      error: None,
      notify: Arc::new(Notify::new()),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestError {
  message: String,
}

impl TestError {
  pub fn new(msg: &str) -> Self {
    Self {
      message: msg.to_string(),
    }
  }
}

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "TestError: {}", self.message)
  }
}

impl std::error::Error for TestError {}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;
  use tokio::runtime::Runtime;
  use tokio::time::sleep;

  fn test_error(msg: &str) -> TestError {
    TestError::new(msg)
  }

  #[test]
  fn test_new_stream() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream: EffectStream<i32, TestError> = EffectStream::new();
      assert!(matches!(stream.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_push_and_next() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      stream.push(1).await.unwrap();
      stream.push(2).await.unwrap();
      stream.push(3).await.unwrap();

      assert!(matches!(stream.next().await, Ok(Some(1))));
      assert!(matches!(stream.next().await, Ok(Some(2))));
      assert!(matches!(stream.next().await, Ok(Some(3))));
      assert!(matches!(stream.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_close() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      stream.push(1).await.unwrap();
      stream.close().await.unwrap();

      assert!(matches!(stream.next().await, Ok(Some(1))));
      assert!(matches!(stream.next().await, Ok(None)));
      assert!(stream.is_closed().await);

      // Should not be able to push after closing
      assert!(matches!(stream.push(2).await, Err(EffectError::Closed)));
      assert!(matches!(stream.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_error_propagation() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      let err = test_error("test error");
      stream.set_error(err.clone()).await.unwrap();

      assert!(matches!(stream.next().await, Err(EffectError::Custom(_))));
      assert!(matches!(stream.next().await, Err(EffectError::Custom(_))));
    });
  }

  #[test]
  fn test_clone() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      stream.push(1).await.unwrap();

      let clone = stream.clone();
      stream.push(2).await.unwrap();

      assert!(matches!(clone.next().await, Ok(Some(1))));
      assert!(matches!(clone.next().await, Ok(Some(2))));
      assert!(matches!(clone.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_concurrent_access() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      let stream_clone = stream.clone();

      let producer = tokio::spawn(async move {
        for i in 0..5 {
          stream.push(i).await.unwrap();
          sleep(Duration::from_millis(10)).await;
        }
        stream.close().await.unwrap();
      });

      let consumer = tokio::spawn(async move {
        let mut values = Vec::new();
        while let Ok(Some(value)) = stream_clone.next().await {
          values.push(value);
        }
        values
      });

      producer.await.unwrap();
      let values = consumer.await.unwrap();
      assert_eq!(values, vec![0, 1, 2, 3, 4]);
    });
  }

  #[test]
  fn test_from_iter() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let values = vec![1, 2, 3];
      let stream = EffectStream::<_, TestError>::from_iter(values.clone());

      for value in values {
        assert!(matches!(stream.next().await, Ok(Some(v)) if v == value));
      }
      assert!(matches!(stream.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_map() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      let mapped = stream.map(|x| x * 2).await;

      mapped.push(1).await.unwrap();
      mapped.push(2).await.unwrap();
      mapped.close().await.unwrap();

      assert!(matches!(mapped.next().await, Ok(Some(2))));
      assert!(matches!(mapped.next().await, Ok(Some(4))));
      assert!(matches!(mapped.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_monad_bind() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      let bound = <EffectStream<_, _> as Monad<_>>::bind(stream.clone(), |x| {
        let new_stream = EffectStream::new();
        let new_stream_clone = new_stream.clone();

        tokio::spawn(async move {
          new_stream_clone.push(x).await.unwrap();
          new_stream_clone.push(x * 2).await.unwrap();
          new_stream_clone.close().await.unwrap();
        });

        new_stream
      });

      stream.push(1).await.unwrap();
      stream.close().await.unwrap();

      sleep(Duration::from_millis(50)).await;

      assert!(matches!(bound.next().await, Ok(Some(1))));
      assert!(matches!(bound.next().await, Ok(Some(2))));
      assert!(matches!(bound.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_empty_stream() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      stream.close().await.unwrap();
      assert!(matches!(stream.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_error_after_values() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      stream.push(1).await.unwrap();
      stream.push(2).await.unwrap();
      let err = test_error("test error");
      stream.set_error(err.clone()).await.unwrap();

      assert!(matches!(stream.next().await, Ok(Some(1))));
      assert!(matches!(stream.next().await, Ok(Some(2))));
      assert!(matches!(stream.next().await, Err(EffectError::Custom(_))));
    });
  }

  #[test]
  fn test_multiple_consumers() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      let clone1 = stream.clone();
      let clone2 = stream.clone();

      stream.push(1).await.unwrap();
      stream.push(2).await.unwrap();
      stream.close().await.unwrap();

      assert!(matches!(clone1.next().await, Ok(Some(1))));
      assert!(matches!(clone2.next().await, Ok(Some(2))));
      assert!(matches!(clone1.next().await, Ok(None)));
      assert!(matches!(clone2.next().await, Ok(None)));
    });
  }

  #[test]
  fn test_stream_trait() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
      let stream = EffectStream::<i32, TestError>::new();
      let mut stream = Box::pin(stream);

      stream.push(1).await.unwrap();
      stream.push(2).await.unwrap();
      stream.close().await.unwrap();

      assert!(matches!(stream.next().await, Some(Ok(1))));
      assert!(matches!(stream.next().await, Some(Ok(2))));
      assert!(matches!(stream.next().await, None));
    });
  }

  #[tokio::test]
  async fn test_bind() {
    let stream = EffectStream::<i32, TestError>::new();
    assert!(matches!(stream.push(1).await, Ok(())));
    assert!(matches!(stream.push(2).await, Ok(())));
    assert!(matches!(stream.close().await, Ok(())));

    let bound = stream
      .bind(|x| {
        let mut s = EffectStream::new();
        s.push(x * 2);
        s.push(x * 3);
        s
      })
      .await;

    assert!(matches!(bound.next().await, Ok(Some(2))));
    assert!(matches!(bound.next().await, Ok(Some(3))));
    assert!(matches!(bound.next().await, Ok(Some(4))));
    assert!(matches!(bound.next().await, Ok(Some(6))));
    assert!(matches!(bound.next().await, Ok(None)));
  }
}
