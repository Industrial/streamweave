use effect_core::{effect::Effect, functor::Functor, monad::Monad};
use futures::{Stream, StreamExt};
use std::{
  iter::FromIterator,
  pin::Pin,
  sync::Arc,
  task::{Context, Poll},
};
use tokio::sync::Mutex;

/// Type alias for the inner stream type used in EffectStream
type InnerStream<T, E> = Pin<Box<dyn Stream<Item = Result<T, E>> + Send + Sync>>;

/// A stream of values that can be transformed using the Effect type system.
pub struct EffectStream<T: Send + Sync + 'static, E: Send + Sync + 'static = std::io::Error> {
  inner: Arc<Mutex<InnerStream<T, E>>>,
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Clone for EffectStream<T, E> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> EffectStream<T, E> {
  /// Creates a new EffectStream from a Stream.
  pub fn from_stream<S>(stream: S) -> Self
  where
    S: Stream<Item = Result<T, E>> + Send + Sync + 'static,
  {
    Self {
      inner: Arc::new(Mutex::new(Box::pin(stream) as InnerStream<T, E>)),
    }
  }

  /// Creates a new EffectStream from a single value.
  pub fn once(value: T) -> Self {
    Self::from_iter(std::iter::once(value))
  }

  /// Converts the stream into an Effect that collects all values into a Vec.
  pub fn collect(self) -> Effect<Vec<T>, E> {
    Effect::new(async move {
      let mut values = Vec::new();
      let inner = self.inner;

      loop {
        let next = {
          let mut stream = inner.lock().await;
          stream.as_mut().next().await
        };
        match next {
          Some(Ok(value)) => values.push(value),
          Some(Err(e)) => return Err(e),
          None => break,
        }
      }

      Ok(values)
    })
  }

  /// Applies a function to each element of the stream.
  pub fn for_each<F>(self, mut f: F) -> Effect<(), E>
  where
    F: FnMut(T) + Send + Sync + 'static,
  {
    Effect::new(async move {
      let inner = self.inner;

      loop {
        let next = {
          let mut stream = inner.lock().await;
          stream.as_mut().next().await
        };
        match next {
          Some(Ok(value)) => f(value),
          Some(Err(e)) => return Err(e),
          None => break,
        }
      }

      Ok(())
    })
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> FromIterator<T> for EffectStream<T, E> {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let vec: Vec<_> = iter.into_iter().collect();
    Self::from_stream(futures::stream::iter(vec.into_iter().map(Ok::<T, E>)))
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Stream for EffectStream<T, E> {
  type Item = Result<T, E>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut stream = futures::executor::block_on(self.inner.lock());
    stream.as_mut().poll_next(cx)
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Functor<T> for EffectStream<T, E> {
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
          let next = {
            let mut stream = inner.lock().await;
            stream.as_mut().next().await
          };
          match next {
            Some(Ok(value)) => {
              let mut f = f.lock().await;
              Some((Ok(f(value)), inner))
            }
            Some(Err(e)) => Some((Err(e), inner)),
            None => None,
          }
        }
      },
    )))
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Monad<T> for EffectStream<T, E> {
  type HigherSelf<U: Send + Sync + 'static> = EffectStream<U, E>;

  fn pure(a: T) -> Self::HigherSelf<T> {
    EffectStream::from_stream(Box::pin(futures::stream::once(async move { Ok(a) })))
  }

  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    let f = Arc::new(Mutex::new(f));
    EffectStream::from_stream(Box::pin(futures::stream::unfold(
      (self.inner, f),
      move |(inner, f)| {
        let f = f.clone();
        async move {
          let next = {
            let mut stream = inner.lock().await;
            stream.as_mut().next().await
          };
          match next {
            Some(Ok(value)) => {
              let mut f_guard = f.lock().await;
              let next_stream = f_guard(value);
              drop(f_guard);

              let next = {
                let mut stream = next_stream.inner.lock().await;
                stream.as_mut().next().await
              };
              match next {
                Some(Ok(value)) => Some((Ok(value), (inner, f))),
                Some(Err(e)) => Some((Err(e), (inner, f))),
                None => None,
              }
            }
            Some(Err(e)) => Some((Err(e), (inner, f))),
            None => None,
          }
        }
      },
    )))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io;

  // Basic operations
  #[tokio::test]
  async fn test_stream_map() {
    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);
    let mapped = Functor::map(stream, move |x| x * 2);
    let results = mapped.collect().await.unwrap();
    assert_eq!(results, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_stream_bind() {
    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);
    let bound = Monad::bind(stream, move |x| EffectStream::from_iter(vec![x, x * 2]));
    let results = bound.collect().await.unwrap();
    assert_eq!(results, vec![1, 2, 2, 4, 3, 6]);
  }

  #[tokio::test]
  async fn test_stream_pure() {
    let stream = EffectStream::<i32, io::Error>::pure(42);
    let results = stream.collect().await.unwrap();
    assert_eq!(results, vec![42]);
  }

  // Error handling
  #[tokio::test]
  async fn test_stream_error() {
    let stream = EffectStream::<i32, io::Error>::from_stream(futures::stream::iter(vec![
      Ok(1),
      Err(io::Error::new(io::ErrorKind::Other, "test error")),
      Ok(3),
    ]));
    let results = stream.collect().await;
    assert!(results.is_err());
  }

  #[tokio::test]
  async fn test_stream_error_map() {
    let stream = EffectStream::<i32, io::Error>::from_stream(futures::stream::iter(vec![
      Ok(1),
      Err(io::Error::new(io::ErrorKind::Other, "test error")),
      Ok(3),
    ]));
    let mapped = Functor::map(stream, move |x| x * 2);
    let results = mapped.collect().await;
    assert!(results.is_err());
  }

  #[tokio::test]
  async fn test_stream_error_bind() {
    let stream = EffectStream::<i32, io::Error>::from_stream(futures::stream::iter(vec![
      Ok(1),
      Err(io::Error::new(io::ErrorKind::Other, "test error")),
      Ok(3),
    ]));
    let bound = Monad::bind(stream, move |x| EffectStream::from_iter(vec![x, x * 2]));
    let results = bound.collect().await;
    assert!(results.is_err());
  }

  // Empty stream tests
  #[tokio::test]
  async fn test_empty_stream() {
    let stream = EffectStream::<i32, io::Error>::from_iter(Vec::<i32>::new());
    let results = stream.collect().await.unwrap();
    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_empty_stream_map() {
    let stream = EffectStream::<i32, io::Error>::from_iter(Vec::<i32>::new());
    let mapped = Functor::map(stream, move |x| x * 2);
    let results = mapped.collect().await.unwrap();
    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_empty_stream_bind() {
    let stream = EffectStream::<i32, io::Error>::from_iter(Vec::<i32>::new());
    let bound = Monad::bind(stream, move |x| EffectStream::from_iter(vec![x, x * 2]));
    let results = bound.collect().await.unwrap();
    assert_eq!(results, Vec::<i32>::new());
  }

  // Functor laws
  #[tokio::test]
  async fn test_functor_identity() {
    // fmap id = id
    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);
    let mapped = Functor::map(stream.clone(), move |x| x);
    let results = mapped.collect().await.unwrap();
    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_functor_composition() {
    // fmap (f . g) = fmap f . fmap g
    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);

    let f = |x| x * 2;
    let g = |x| x + 1;

    // (f . g)(x)
    let composed = Functor::map(stream.clone(), move |x| f(g(x)));
    let composed_results = composed.collect().await.unwrap();

    // fmap f . fmap g
    let chained = Functor::map(Functor::map(stream, g), f);
    let chained_results = chained.collect().await.unwrap();

    assert_eq!(composed_results, chained_results);
  }

  // Monad laws
  #[tokio::test]
  async fn test_monad_left_identity() {
    // return a >>= f = f a
    let a = 42;
    let f = |x| EffectStream::<i32, io::Error>::from_iter(vec![x, x * 2]);

    let left = Monad::bind(EffectStream::pure(a), f);
    let right = f(a);

    let left_results = left.collect().await.unwrap();
    let right_results = right.collect().await.unwrap();

    assert_eq!(left_results, right_results);
  }

  #[tokio::test]
  async fn test_monad_right_identity() {
    // m >>= return = m
    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);

    let bound = Monad::bind(stream.clone(), EffectStream::pure);

    let left_results = stream.collect().await.unwrap();
    let right_results = bound.collect().await.unwrap();

    assert_eq!(left_results, right_results);
  }

  #[tokio::test]
  async fn test_monad_associativity() {
    // (m >>= f) >>= g = m >>= (\x -> f x >>= g)
    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);

    let f = |x| EffectStream::from_iter(vec![x, x * 2]);
    let g = |x| EffectStream::from_iter(vec![x, x + 1]);

    // (m >>= f) >>= g
    let left = Monad::bind(Monad::bind(stream.clone(), f), g);

    // m >>= (\x -> f x >>= g)
    let right = Monad::bind(stream, move |x| Monad::bind(f(x), g));

    let left_results = left.collect().await.unwrap();
    let right_results = right.collect().await.unwrap();

    assert_eq!(left_results, right_results);
  }

  // Complex type tests
  #[derive(Debug, Clone, PartialEq)]
  struct TestStruct {
    value: i32,
    text: String,
  }

  #[tokio::test]
  async fn test_complex_type() {
    let items = vec![
      TestStruct {
        value: 1,
        text: "one".to_string(),
      },
      TestStruct {
        value: 2,
        text: "two".to_string(),
      },
    ];

    let stream = EffectStream::<TestStruct, io::Error>::from_iter(items.clone());
    let results = stream.collect().await.unwrap();
    assert_eq!(results, items);
  }

  #[tokio::test]
  async fn test_complex_type_map() {
    let items = vec![
      TestStruct {
        value: 1,
        text: "one".to_string(),
      },
      TestStruct {
        value: 2,
        text: "two".to_string(),
      },
    ];

    let stream = EffectStream::<TestStruct, io::Error>::from_iter(items);
    let mapped = Functor::map(stream, move |x| x.value);
    let results = mapped.collect().await.unwrap();
    assert_eq!(results, vec![1, 2]);
  }

  #[tokio::test]
  async fn test_complex_type_bind() {
    let items = vec![
      TestStruct {
        value: 1,
        text: "one".to_string(),
      },
      TestStruct {
        value: 2,
        text: "two".to_string(),
      },
    ];

    let stream = EffectStream::<TestStruct, io::Error>::from_iter(items);
    let bound = Monad::bind(stream, move |x| {
      EffectStream::from_iter(vec![
        TestStruct {
          value: x.value,
          text: x.text.clone(),
        },
        TestStruct {
          value: x.value * 2,
          text: format!("{} doubled", x.text),
        },
      ])
    });

    let results = bound.collect().await.unwrap();
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].value, 1);
    assert_eq!(results[1].value, 2);
    assert_eq!(results[2].value, 2);
    assert_eq!(results[3].value, 4);
  }

  // For_each tests
  #[tokio::test]
  async fn test_for_each() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let sum = Arc::new(AtomicI32::new(0));
    let sum_clone = sum.clone();

    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);
    stream
      .for_each(move |x| {
        sum_clone.fetch_add(x, Ordering::SeqCst);
      })
      .await
      .unwrap();

    assert_eq!(sum.load(Ordering::SeqCst), 6);
  }

  #[tokio::test]
  async fn test_for_each_empty() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let counter = Arc::new(AtomicI32::new(0));
    let counter_clone = counter.clone();

    let stream = EffectStream::<i32, io::Error>::from_iter(Vec::<i32>::new());
    stream
      .for_each(move |_| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
      })
      .await
      .unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 0);
  }

  // Clone tests
  #[tokio::test]
  async fn test_clone() {
    let stream = EffectStream::<i32, io::Error>::from_iter(vec![1, 2, 3]);
    let cloned = stream.clone();

    let original_results = stream.collect().await.unwrap();
    let cloned_results = cloned.collect().await.unwrap();

    assert_eq!(original_results, cloned_results);
  }

  #[tokio::test]
  async fn test_clone_empty() {
    let stream = EffectStream::<i32, io::Error>::from_iter(Vec::<i32>::new());
    let cloned = stream.clone();

    let original_results = stream.collect().await.unwrap();
    let cloned_results = cloned.collect().await.unwrap();

    assert_eq!(original_results, cloned_results);
  }
}
