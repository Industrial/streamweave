use effect_stream::{EffectResult, EffectStream, EffectStreamSource};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Error type for StringStream operations
#[derive(Debug, Clone)]
pub struct StringError;

/// A stream source that produces characters from a String
pub struct StringStreamSource {
  string: Arc<Mutex<String>>,
}

impl StringStreamSource {
  /// Create a new stream from a String
  pub fn new(string: String) -> Self {
    Self {
      string: Arc::new(Mutex::new(string)),
    }
  }

  /// Create a new stream from a string slice
  pub fn from_string_slice(s: &str) -> Self {
    Self::new(s.to_string())
  }

  /// Get the length of the String
  pub async fn len(&self) -> usize {
    let string = self.string.lock().await;
    string.len()
  }

  /// Check if the String is empty
  pub async fn is_empty(&self) -> bool {
    let string = self.string.lock().await;
    string.is_empty()
  }
}

impl EffectStreamSource<char, StringError> for StringStreamSource {
  type Stream = Pin<
    Box<
      dyn Future<Output = EffectResult<EffectStream<char, StringError>, StringError>>
        + Send
        + 'static,
    >,
  >;

  fn source(&self) -> Self::Stream {
    let string = self.string.clone();
    Box::pin(async move {
      let string = string.lock().await;
      let stream = EffectStream::new();
      let stream_clone = stream.clone();
      let string_clone = string.clone();

      tokio::spawn(async move {
        for ch in string_clone.chars() {
          stream_clone.push(ch).await.unwrap();
        }
        stream_clone.close().await.unwrap();
      });

      Ok(stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::time::{sleep, Duration};

  #[tokio::test]
  async fn test_string_stream_basic() {
    let source = StringStreamSource::new("hello".to_string());
    let stream = source.source().await.unwrap();
    let mut values = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      values.push(value);
    }
    assert_eq!(values, vec!['h', 'e', 'l', 'l', 'o']);
  }

  #[tokio::test]
  async fn test_string_stream_empty() {
    let source = StringStreamSource::new(String::new());
    let stream = source.source().await.unwrap();
    let mut values = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      values.push(value);
    }
    assert!(values.is_empty());
  }

  #[tokio::test]
  async fn test_string_stream_from_string_slice() {
    let source = StringStreamSource::from_string_slice("hello");
    let stream = source.source().await.unwrap();
    let mut values = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      values.push(value);
    }
    assert_eq!(values, vec!['h', 'e', 'l', 'l', 'o']);
  }

  #[tokio::test]
  async fn test_len_and_is_empty() {
    let source = StringStreamSource::new("hello".to_string());
    assert_eq!(source.len().await, 5);
    assert!(!source.is_empty().await);

    let empty_source = StringStreamSource::new(String::new());
    assert_eq!(empty_source.len().await, 0);
    assert!(empty_source.is_empty().await);
  }

  #[tokio::test]
  async fn test_string_stream_unicode() {
    let source = StringStreamSource::new("Hello, 世界! 🌍".to_string());
    let stream = source.source().await.unwrap();
    let mut values = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      values.push(value);
    }
    assert_eq!(values.iter().collect::<String>(), "Hello, 世界! 🌍");
  }

  #[tokio::test]
  async fn test_string_stream_concurrent_access() {
    let source = StringStreamSource::new("test".to_string());

    // Create multiple streams from the same source
    let stream1 = source.source().await.unwrap();
    let stream2 = source.source().await.unwrap();

    let handle1 = tokio::spawn(async move {
      let mut values = Vec::new();
      while let Ok(Some(value)) = stream1.next().await {
        values.push(value);
      }
      values
    });

    let handle2 = tokio::spawn(async move {
      let mut values = Vec::new();
      while let Ok(Some(value)) = stream2.next().await {
        values.push(value);
      }
      values
    });

    let values1 = handle1.await.unwrap();
    let values2 = handle2.await.unwrap();

    assert_eq!(values1, vec!['t', 'e', 's', 't']);
    assert_eq!(values2, vec!['t', 'e', 's', 't']);
  }

  #[tokio::test]
  async fn test_string_stream_large() {
    let large_string = "a".repeat(10000);
    let source = StringStreamSource::new(large_string.clone());
    let stream = source.source().await.unwrap();
    let mut values = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      values.push(value);
    }
    assert_eq!(values.len(), 10000);
    assert!(values.iter().all(|&c| c == 'a'));
  }

  #[tokio::test]
  async fn test_string_stream_delayed_consumption() {
    let source = StringStreamSource::new("test".to_string());
    let stream = source.source().await.unwrap();
    let mut values = Vec::new();

    // Delay between reading each character
    while let Ok(Some(value)) = stream.next().await {
      values.push(value);
      sleep(Duration::from_millis(1)).await;
    }

    assert_eq!(values, vec!['t', 'e', 's', 't']);
  }

  #[tokio::test]
  async fn test_string_stream_multiple_sources() {
    let source1 = StringStreamSource::new("Hello".to_string());
    let source2 = StringStreamSource::new(", World!".to_string());

    let stream1 = source1.source().await.unwrap();
    let stream2 = source2.source().await.unwrap();

    let mut values = Vec::new();

    // Consume from first stream
    while let Ok(Some(value)) = stream1.next().await {
      values.push(value);
    }

    // Consume from second stream
    while let Ok(Some(value)) = stream2.next().await {
      values.push(value);
    }

    assert_eq!(values.iter().collect::<String>(), "Hello, World!");
  }
}
