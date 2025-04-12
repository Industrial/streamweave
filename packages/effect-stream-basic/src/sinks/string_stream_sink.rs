use effect_stream::{EffectResult, EffectStream, EffectStreamSink};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Error type for StringStream operations
#[derive(Debug, Clone)]
pub struct StringError;

/// A sink that collects characters into a String
pub struct StringStreamSink {
  string: Arc<Mutex<String>>,
}

impl StringStreamSink {
  /// Create a new sink
  pub fn new() -> Self {
    Self {
      string: Arc::new(Mutex::new(String::new())),
    }
  }

  /// Get the collected string
  pub async fn into_string(self) -> String {
    let string = self.string.lock().await;
    string.clone()
  }
}

impl Default for StringStreamSink {
  fn default() -> Self {
    Self::new()
  }
}

impl EffectStreamSink<char, StringError> for StringStreamSink {
  type Future = Pin<Box<dyn Future<Output = EffectResult<(), StringError>> + Send + 'static>>;

  fn consume(&self, stream: EffectStream<char, StringError>) -> Self::Future {
    let stream_clone = stream.clone();
    let string = self.string.clone();

    Box::pin(async move {
      let stream = stream_clone;
      while let Ok(Some(ch)) = stream.next().await {
        let mut string = string.lock().await;
        string.push(ch);
      }
      Ok(())
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::time::{sleep, Duration};

  #[tokio::test]
  async fn test_string_stream_sink_basic() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push('h').await.unwrap();
      stream_clone.push('e').await.unwrap();
      stream_clone.push('l').await.unwrap();
      stream_clone.push('l').await.unwrap();
      stream_clone.push('o').await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "hello");
  }

  #[tokio::test]
  async fn test_string_stream_sink_empty() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "");
  }

  #[tokio::test]
  async fn test_string_stream_sink_unicode() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for ch in "Hello, 世界! 🌍".chars() {
        stream_clone.push(ch).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "Hello, 世界! 🌍");
  }

  #[tokio::test]
  async fn test_string_stream_sink_concurrent() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      let hello = "Hello".chars();
      let world = ", World!".chars();

      // Write "Hello" first
      for ch in hello {
        stream_clone.push(ch).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }

      // Then write ", World!"
      for ch in world {
        stream_clone.push(ch).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }

      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "Hello, World!");
  }

  #[tokio::test]
  async fn test_string_stream_sink_large() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();
    let large_string = "a".repeat(10000);

    tokio::spawn(async move {
      for ch in large_string.chars() {
        stream_clone.push(ch).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string.len(), 10000);
    assert!(string.chars().all(|c| c == 'a'));
  }

  #[tokio::test]
  async fn test_string_stream_sink_interleaved() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for (i, ch) in "Hello, World!".chars().enumerate() {
        if i % 2 == 0 {
          sleep(Duration::from_millis(1)).await;
        }
        stream_clone.push(ch).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "Hello, World!");
  }

  #[tokio::test]
  async fn test_string_stream_sink_multiple_consumers() {
    // Create two independent streams
    let stream1 = EffectStream::<char, StringError>::new();
    let stream2 = EffectStream::<char, StringError>::new();
    let producer_stream1 = stream1.clone();
    let producer_stream2 = stream2.clone();

    // Producer task for first stream
    let producer1 = tokio::spawn(async move {
      for ch in "Hello, World!".chars() {
        producer_stream1.push(ch).await.unwrap();
      }
      producer_stream1.close().await.unwrap();
    });

    // Producer task for second stream
    let producer2 = tokio::spawn(async move {
      for ch in "Hello, World!".chars() {
        producer_stream2.push(ch).await.unwrap();
      }
      producer_stream2.close().await.unwrap();
    });

    // Consumer tasks
    let consumer1 = tokio::spawn(async move {
      let sink = StringStreamSink::new();
      sink.consume(stream1).await.unwrap();
      sink.into_string().await
    });

    let consumer2 = tokio::spawn(async move {
      let sink = StringStreamSink::new();
      sink.consume(stream2).await.unwrap();
      sink.into_string().await
    });

    // Wait for producers to finish
    let (_, _) = tokio::join!(producer1, producer2);

    // Then wait for consumers
    let (result1, result2) = tokio::join!(consumer1, consumer2);
    assert_eq!(result1.unwrap(), "Hello, World!");
    assert_eq!(result2.unwrap(), "Hello, World!");
  }

  #[tokio::test]
  async fn test_string_stream_sink_error_handling() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      // Push some valid characters
      for ch in "Hello".chars() {
        stream_clone.push(ch).await.unwrap();
      }
      // Close the stream
      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    let result = sink.consume(stream).await;
    assert!(result.is_ok());
    let string = sink.into_string().await;
    assert_eq!(string, "Hello");
  }

  #[tokio::test]
  async fn test_string_stream_sink_mixed_unicode() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      // Mix of ASCII, CJK, and emoji characters
      let mixed = "Hello 你好 🌍 World! 世界 🌎";
      for ch in mixed.chars() {
        stream_clone.push(ch).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "Hello 你好 🌍 World! 世界 🌎");
  }

  #[tokio::test]
  async fn test_string_stream_sink_concurrent_mixed() {
    let stream = EffectStream::<char, StringError>::new();
    let producer_stream = stream.clone();

    // Single producer writing mixed content
    tokio::spawn(async move {
      // Write ASCII first
      for ch in "Hello".chars() {
        producer_stream.push(ch).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      // Then write Unicode
      for ch in " 世界 🌍".chars() {
        producer_stream.push(ch).await.unwrap();
        sleep(Duration::from_millis(1)).await;
      }
      producer_stream.close().await.unwrap();
    });

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "Hello 世界 🌍");
  }

  #[tokio::test]
  async fn test_string_stream_sink_delayed_consumption() {
    let stream = EffectStream::<char, StringError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for ch in "Hello, World!".chars() {
        stream_clone.push(ch).await.unwrap();
        sleep(Duration::from_millis(10)).await;
      }
      stream_clone.close().await.unwrap();
    });

    // Wait a bit before consuming
    sleep(Duration::from_millis(50)).await;

    let sink = StringStreamSink::new();
    sink.consume(stream).await.unwrap();
    let string = sink.into_string().await;
    assert_eq!(string, "Hello, World!");
  }
}
