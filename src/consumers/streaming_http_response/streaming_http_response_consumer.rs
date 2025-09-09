use super::response_chunk::ResponseChunk;
use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;

/// Streaming HTTP response consumer for processing response chunks
#[derive(Debug, Clone)]
pub struct StreamingHttpResponseConsumer {
  pub chunk_sender: tokio::sync::mpsc::Sender<ResponseChunk>,
  pub config: ConsumerConfig<ResponseChunk>,
}

impl StreamingHttpResponseConsumer {
  pub fn new() -> (Self, tokio::sync::mpsc::Receiver<ResponseChunk>) {
    let (tx, rx) = tokio::sync::mpsc::channel(100); // Buffer size of 100 chunks
    let consumer = Self {
      chunk_sender: tx,
      config: ConsumerConfig::default(),
    };
    (consumer, rx)
  }

  pub fn with_buffer_size(
    buffer_size: usize,
  ) -> (Self, tokio::sync::mpsc::Receiver<ResponseChunk>) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer_size);
    let consumer = Self {
      chunk_sender: tx,
      config: ConsumerConfig::default(),
    };
    (consumer, rx)
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<ResponseChunk>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bytes::Bytes;
  use http::StatusCode;

  #[tokio::test]
  async fn test_new() {
    let (consumer, mut receiver) = StreamingHttpResponseConsumer::new();

    // Send a test chunk
    let chunk = ResponseChunk::body(Bytes::from("test"));
    let _ = consumer.chunk_sender.send(chunk.clone()).await;

    // Receive the chunk
    let received = receiver.recv().await.unwrap();
    assert_eq!(received, chunk);
  }

  #[tokio::test]
  async fn test_with_buffer_size() {
    let buffer_size = 50;
    let (consumer, mut receiver) = StreamingHttpResponseConsumer::with_buffer_size(buffer_size);

    // Send multiple chunks to test buffer
    for i in 0..10 {
      let chunk = ResponseChunk::body(Bytes::from(format!("chunk {}", i)));
      let _ = consumer.chunk_sender.send(chunk).await;
    }

    // Receive chunks
    for i in 0..10 {
      let received = receiver.recv().await.unwrap();
      match received {
        ResponseChunk::Body(body) => {
          assert_eq!(body, Bytes::from(format!("chunk {}", i)));
        }
        _ => panic!("Expected Body chunk"),
      }
    }
  }

  #[test]
  fn test_with_error_strategy() {
    use crate::error::ErrorAction;
    let strategy = ErrorStrategy::<ResponseChunk>::new_custom(|_| ErrorAction::Skip);
    let (consumer, _) = StreamingHttpResponseConsumer::new();
    let consumer = consumer.with_error_strategy(strategy.clone());

    assert_eq!(consumer.config.error_strategy, strategy);
  }

  #[test]
  fn test_with_name() {
    let name = "test_consumer".to_string();
    let (consumer, _) = StreamingHttpResponseConsumer::new();
    let consumer = consumer.with_name(name.clone());

    assert_eq!(consumer.config.name, name);
  }

  #[tokio::test]
  async fn test_chunk_types() {
    let (consumer, mut receiver) = StreamingHttpResponseConsumer::new();

    // Test header chunk
    let header_chunk = ResponseChunk::header(StatusCode::OK, http::HeaderMap::new());
    let _ = consumer.chunk_sender.send(header_chunk.clone()).await;
    let received = receiver.recv().await.unwrap();
    assert_eq!(received, header_chunk);

    // Test body chunk
    let body_chunk = ResponseChunk::body(Bytes::from("test body"));
    let _ = consumer.chunk_sender.send(body_chunk.clone()).await;
    let received = receiver.recv().await.unwrap();
    assert_eq!(received, body_chunk);

    // Test end chunk
    let end_chunk = ResponseChunk::end();
    let _ = consumer.chunk_sender.send(end_chunk.clone()).await;
    let received = receiver.recv().await.unwrap();
    assert_eq!(received, end_chunk);

    // Test error chunk
    let error_chunk = ResponseChunk::error(StatusCode::BAD_REQUEST, "test error".to_string());
    let _ = consumer.chunk_sender.send(error_chunk.clone()).await;
    let received = receiver.recv().await.unwrap();
    assert_eq!(received, error_chunk);
  }
}
