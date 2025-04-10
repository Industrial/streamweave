use effect_core::{Functor, Monad};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;

use crate::error::{EffectError, EffectResult};

/// A stream of values that are produced by effects
pub struct EffectStream<T, E> {
  inner: Arc<Mutex<InnerStream<T, E>>>,
}

struct InnerStream<T, E> {
  values: Vec<T>,
  error: Option<EffectError<E>>,
  is_closed: bool,
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

impl<T, E> EffectStream<T, E> {
  /// Create a new empty stream
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(InnerStream {
        values: Vec::new(),
        error: None,
        is_closed: false,
      })),
    }
  }

  /// Create a stream from a futures Stream
  pub fn from_stream<S>(stream: S) -> Self
  where
    S: Stream<Item = EffectResult<T, E>> + Send + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    let inner = Arc::new(Mutex::new(InnerStream {
      values: Vec::new(),
      error: None,
      is_closed: false,
    }));

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
    let mut inner = self.inner.lock().await;
    if inner.is_closed {
      return Err(EffectError::Closed);
    }
    inner.values.push(value);
    Ok(())
  }

  /// Close the stream
  pub async fn close(&self) -> EffectResult<(), E> {
    let mut inner = self.inner.lock().await;
    if inner.is_closed {
      return Err(EffectError::Closed);
    }
    inner.is_closed = true;
    Ok(())
  }

  /// Check if the stream is closed
  pub async fn is_closed(&self) -> bool {
    let inner = self.inner.lock().await;
    inner.is_closed
  }

  /// Get the next value from the stream
  pub async fn next(&self) -> EffectResult<Option<T>, E>
  where
    E: Clone,
  {
    let mut inner = self.inner.lock().await;
    if let Some(err) = inner.error.clone() {
      return Err(err);
    }
    if inner.is_closed && inner.values.is_empty() {
      return Ok(None);
    }
    Ok(inner.values.pop())
  }

  /// Set an error on the stream
  pub async fn set_error(&self, error: EffectError<E>) -> EffectResult<(), E> {
    let mut inner = self.inner.lock().await;
    if inner.is_closed {
      return Err(EffectError::Closed);
    }
    inner.error = Some(error);
    Ok(())
  }
}

impl<T, E> Stream for EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  type Item = EffectResult<T, E>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mutex = Arc::clone(&self.inner);
    let mut inner = match mutex.try_lock() {
      Ok(inner) => inner,
      Err(_) => {
        cx.waker().wake_by_ref();
        return Poll::Pending;
      }
    };

    if let Some(err) = inner.error.clone() {
      return Poll::Ready(Some(Err(err)));
    }

    if inner.is_closed && inner.values.is_empty() {
      return Poll::Ready(None);
    }

    if let Some(value) = inner.values.pop() {
      Poll::Ready(Some(Ok(value)))
    } else {
      cx.waker().wake_by_ref();
      Poll::Pending
    }
  }
}

impl<T, E> Clone for EffectStream<T, E> {
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
    }
  }
}

impl<T, E> Default for EffectStream<T, E> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T, E> FromIterator<T> for EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let values: Vec<T> = iter.into_iter().collect();
    Self {
      inner: Arc::new(Mutex::new(InnerStream {
        values,
        error: None,
        is_closed: false,
      })),
    }
  }
}

impl<T, E> Functor<T> for EffectStream<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  type HigherSelf<U: Send + Sync + 'static> = EffectStream<U, E>;

  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    let f = Arc::new(Mutex::new(f));
    EffectStream::from_stream(Box::pin(futures::stream::unfold(
      self.inner,
      move |inner| {
        let f = f.clone();
        async move {
          let (value, err) = {
            let mut stream = inner.lock().await;
            if let Some(err) = stream.error.clone() {
              (None, Some(err))
            } else if stream.is_closed && stream.values.is_empty() {
              (None, None)
            } else {
              (stream.values.pop(), None)
            }
          };

          match (value, err) {
            (Some(value), None) => {
              let mut f = f.lock().await;
              Some((Ok(f(value)), inner))
            }
            (_, Some(err)) => Some((Err(err), inner)),
            _ => None,
          }
        }
      },
    )))
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
  let mut stream_guard = stream.lock().await;
  if let Some(err) = stream_guard.error.clone() {
    return Some(Err(err));
  }
  if stream_guard.is_closed && stream_guard.values.is_empty() {
    None
  } else {
    stream_guard.values.pop().map(Ok)
  }
}

mod tests {
  use super::*;
  use futures::StreamExt;
  use tokio::test;

  #[test]
  async fn test_new() {
    let stream = EffectStream::<i32, String>::new();
    assert!(!stream.is_closed().await);
    assert_eq!(stream.next().await, Ok(None));
  }

  #[test]
  async fn test_from_iter() {
    let stream = EffectStream::from_iter(vec![1, 2, 3]);
    assert_eq!(stream.next().await, Ok(Some(3)));
    assert_eq!(stream.next().await, Ok(Some(2)));
    assert_eq!(stream.next().await, Ok(Some(1)));
    assert_eq!(stream.next().await, Ok(None));
  }

  #[test]
  async fn test_from_stream() {
    let input = futures::stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let stream = EffectStream::from_stream(input);
    assert_eq!(stream.next().await, Ok(Some(3)));
    assert_eq!(stream.next().await, Ok(Some(2)));
    assert_eq!(stream.next().await, Ok(Some(1)));
    assert_eq!(stream.next().await, Ok(None));
  }

  #[test]
  async fn test_push_and_next() {
    let stream = EffectStream::new();
    assert!(stream.push(1).await.is_ok());
    assert!(stream.push(2).await.is_ok());
    assert_eq!(stream.next().await, Ok(Some(2)));
    assert_eq!(stream.next().await, Ok(Some(1)));
    assert_eq!(stream.next().await, Ok(None));
  }

  #[test]
  async fn test_close() {
    let stream = EffectStream::new();
    assert!(stream.push(1).await.is_ok());
    assert!(stream.close().await.is_ok());
    assert!(stream.is_closed().await);
    assert!(stream.push(2).await.is_err());
    assert_eq!(stream.next().await, Ok(Some(1)));
    assert_eq!(stream.next().await, Ok(None));
  }

  #[test]
  async fn test_set_error() {
    let stream = EffectStream::new();
    assert!(stream.push(1).await.is_ok());
    assert!(stream
      .set_error(EffectError::processing("test error"))
      .await
      .is_ok());
    assert!(stream.push(2).await.is_err());
    assert_eq!(
      stream.next().await,
      Err(EffectError::processing("test error"))
    );
  }

  #[test]
  async fn test_map() {
    let stream = EffectStream::from_iter(vec![1, 2, 3]);
    let mapped = stream.map(|x| x * 2);
    assert_eq!(mapped.next().await, Ok(Some(6)));
    assert_eq!(mapped.next().await, Ok(Some(4)));
    assert_eq!(mapped.next().await, Ok(Some(2)));
    assert_eq!(mapped.next().await, Ok(None));
  }

  #[test]
  async fn test_map_with_error() {
    let stream = EffectStream::from_iter(vec![1, 2, 3]);
    let stream = stream.map(|x| if x == 2 { panic!("test error") } else { x * 2 });
    assert_eq!(stream.next().await, Ok(Some(6)));
    assert!(stream.next().await.is_err());
  }

  #[test]
  async fn test_bind() {
    let stream = EffectStream::from_iter(vec![1, 2, 3]);
    let bound = stream.bind(|x| EffectStream::from_iter(vec![x, x * 2]));
    assert_eq!(bound.next().await, Ok(Some(6)));
    assert_eq!(bound.next().await, Ok(Some(3)));
    assert_eq!(bound.next().await, Ok(Some(4)));
    assert_eq!(bound.next().await, Ok(Some(2)));
    assert_eq!(bound.next().await, Ok(Some(2)));
    assert_eq!(bound.next().await, Ok(Some(1)));
    assert_eq!(bound.next().await, Ok(None));
  }

  #[test]
  async fn test_bind_with_error() {
    let stream = EffectStream::from_iter(vec![1, 2, 3]);
    let bound = stream.bind(|x| {
      if x == 2 {
        EffectStream::from_iter(vec![Ok(4), Err(EffectError::processing("test error"))])
      } else {
        EffectStream::from_iter(vec![Ok(x * 2)])
      }
    });
    assert_eq!(bound.next().await, Ok(Some(6)));
    assert_eq!(bound.next().await, Ok(Some(4)));
    assert_eq!(
      bound.next().await,
      Err(EffectError::processing("test error"))
    );
  }

  #[test]
  async fn test_pure() {
    let stream = EffectStream::pure(42);
    assert_eq!(stream.next().await, Ok(Some(42)));
    assert_eq!(stream.next().await, Ok(None));
  }

  #[test]
  async fn test_clone() {
    let stream = EffectStream::from_iter(vec![1, 2, 3]);
    let clone = stream.clone();
    assert_eq!(stream.next().await, Ok(Some(3)));
    assert_eq!(clone.next().await, Ok(Some(3)));
    assert_eq!(stream.next().await, Ok(Some(2)));
    assert_eq!(clone.next().await, Ok(Some(2)));
  }

  #[test]
  async fn test_stream_trait() {
    let stream = EffectStream::from_iter(vec![1, 2, 3]);
    let mut stream = Box::pin(stream);
    assert_eq!(stream.next().await, Some(Ok(3)));
    assert_eq!(stream.next().await, Some(Ok(2)));
    assert_eq!(stream.next().await, Some(Ok(1)));
    assert_eq!(stream.next().await, None);
  }

  #[test]
  async fn test_concurrent_access() {
    let stream = EffectStream::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      assert!(stream_clone.push(1).await.is_ok());
      assert!(stream_clone.push(2).await.is_ok());
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    assert_eq!(stream.next().await, Ok(Some(2)));
    assert_eq!(stream.next().await, Ok(Some(1)));
  }

  #[test]
  async fn test_error_propagation() {
    let stream = EffectStream::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      assert!(stream_clone.push(1).await.is_ok());
      assert!(stream_clone
        .set_error(EffectError::processing("test error"))
        .await
        .is_ok());
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    assert_eq!(stream.next().await, Ok(Some(1)));
    assert_eq!(
      stream.next().await,
      Err(EffectError::processing("test error"))
    );
  }
}
