use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct WindowTransformer<T> {
  window_size: usize,
  buffer: Vec<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> WindowTransformer<T>
where
  T: Clone + Send + 'static,
{
  pub fn new(window_size: usize) -> Result<Self, WindowError> {
    if window_size == 0 {
      return Err(WindowError::ZeroWindowSize);
    }
    Ok(Self {
      window_size,
      buffer: Vec::with_capacity(window_size),
      _phantom: std::marker::PhantomData,
    })
  }
}

#[derive(Debug)]
pub enum WindowError {
  Transform(TransformError),
  ZeroWindowSize,
  InsufficientItems,
  BufferError(String),
}

impl fmt::Display for WindowError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      WindowError::Transform(e) => write!(f, "{}", e),
      WindowError::ZeroWindowSize => write!(f, "Window size must be greater than 0"),
      WindowError::InsufficientItems => write!(f, "Not enough items to fill window"),
      WindowError::BufferError(msg) => write!(f, "Buffer operation failed: {}", msg),
    }
  }
}

impl Error for WindowError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      WindowError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<T> crate::traits::error::Error for WindowTransformer<T>
where
  T: Send + 'static,
{
  type Error = WindowError;
}

impl<T> Input for WindowTransformer<T>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<T> Output for WindowTransformer<T>
where
  T: Send + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<T> Transformer for WindowTransformer<T>
where
  T: Clone + Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let window_size = self.window_size;
    let mut buffer = Vec::with_capacity(window_size);

    Box::pin(futures::stream::unfold(
      (input, buffer),
      move |(mut input, mut buf)| async move {
        if window_size == 0 {
          return Some((Err(WindowError::ZeroWindowSize), (input, buf)));
        }

        match input.next().await {
          Some(Ok(item)) => {
            buf.push(item);
            if buf.len() > window_size {
              buf.remove(0);
            }
            Some((Ok(buf.clone()), (input, buf)))
          }
          Some(Err(e)) => Some((Err(e), (input, buf))),
          None => None,
        }
      },
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_window_basic() {
    let mut transformer = WindowTransformer::new(3).unwrap();
    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<i32>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec![
        vec![1],
        vec![1, 2],
        vec![1, 2, 3],
        vec![2, 3, 4],
        vec![3, 4, 5],
      ]
    );
  }

  #[tokio::test]
  async fn test_window_empty_input() {
    let mut transformer = WindowTransformer::new(3).unwrap();
    let input = Vec::<i32>::new();
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<i32>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_window_size_one() {
    let mut transformer = WindowTransformer::new(1).unwrap();
    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<i32>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
  }

  #[tokio::test]
  async fn test_window_zero_size() {
    let result = WindowTransformer::<i32>::new(0);
    assert!(matches!(result, Err(WindowError::ZeroWindowSize)));
  }

  #[tokio::test]
  async fn test_window_with_strings() {
    let mut transformer = WindowTransformer::new(2).unwrap();
    let input = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<String>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec![
        vec!["a".to_string()],
        vec!["a".to_string(), "b".to_string()],
        vec!["b".to_string(), "c".to_string()],
      ]
    );
  }

  #[tokio::test]
  async fn test_window_reuse() {
    let mut transformer = WindowTransformer::new(2).unwrap();

    let input1 = vec![1, 2, 3];
    let input_stream1 = Box::pin(stream::iter(input1.into_iter().map(Ok)));
    let result1: Vec<Vec<i32>> = transformer
      .transform(input_stream1)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result1, vec![vec![1], vec![1, 2], vec![2, 3]]);

    let input2 = vec![4, 5, 6];
    let input_stream2 = Box::pin(stream::iter(input2.into_iter().map(Ok)));
    let result2: Vec<Vec<i32>> = transformer
      .transform(input_stream2)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result2, vec![vec![4], vec![4, 5], vec![5, 6]]);
  }

  #[tokio::test]
  async fn test_window_error_propagation() {
    let mut transformer = WindowTransformer::new(2).unwrap();
    let input = vec![
      Ok(1),
      Ok(2),
      Err(WindowError::BufferError("test error".to_string())),
      Ok(3),
    ];
    let input_stream = Box::pin(stream::iter(input));

    let result = transformer
      .transform(input_stream)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      WindowError::BufferError(msg) => assert_eq!(msg, "test error"),
      _ => panic!("Expected BufferError"),
    }
  }
}
