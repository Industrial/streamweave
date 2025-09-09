use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::chunked_transfer::chunked_transfer_transformer::ChunkedTransferTransformer;
use crate::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;

#[async_trait]
impl Transformer for ChunkedTransferTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(
      input
        .flat_map(|chunk| {
          let chunk_size = chunk.len();
          let hex_size = format!("{:X}", chunk_size);

          futures::stream::iter(vec![
            Bytes::from(format!("{}\r\n", hex_size)),
            chunk,
            Bytes::from("\r\n"),
          ])
        })
        .chain(futures::stream::once(async move {
          Bytes::from("0\r\n\r\n") // End chunk
        })),
    )
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "chunked_transfer_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tokio_stream::wrappers::ReceiverStream;

  #[tokio::test]
  async fn test_chunked_transfer_basic() {
    let mut transformer = ChunkedTransferTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send a chunk
    let chunk = Bytes::from("Hello World");
    tx.send(chunk).await.unwrap();
    drop(tx);

    // Check output
    let mut chunks = Vec::new();
    while let Some(chunk) = output.next().await {
      chunks.push(chunk);
    }

    // Should have: size, data, CRLF, end chunk
    assert_eq!(chunks.len(), 4);

    // First chunk should be size
    assert_eq!(chunks[0], Bytes::from("B\r\n")); // "B" in hex = 11 bytes

    // Second chunk should be data
    assert_eq!(chunks[1], Bytes::from("Hello World"));

    // Third chunk should be CRLF
    assert_eq!(chunks[2], Bytes::from("\r\n"));

    // Fourth chunk should be end marker
    assert_eq!(chunks[3], Bytes::from("0\r\n\r\n"));
  }

  #[tokio::test]
  async fn test_chunked_transfer_multiple_chunks() {
    let mut transformer = ChunkedTransferTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send multiple chunks
    tx.send(Bytes::from("Hello")).await.unwrap();
    tx.send(Bytes::from("World")).await.unwrap();
    drop(tx);

    // Check output
    let mut chunks = Vec::new();
    while let Some(chunk) = output.next().await {
      chunks.push(chunk);
    }

    // Should have: size1, data1, CRLF, size2, data2, CRLF, end chunk
    assert_eq!(chunks.len(), 7);

    // First chunk size (5 bytes)
    assert_eq!(chunks[0], Bytes::from("5\r\n"));

    // First data
    assert_eq!(chunks[1], Bytes::from("Hello"));

    // CRLF
    assert_eq!(chunks[2], Bytes::from("\r\n"));

    // Second chunk size (5 bytes)
    assert_eq!(chunks[3], Bytes::from("5\r\n"));

    // Second data
    assert_eq!(chunks[4], Bytes::from("World"));

    // CRLF
    assert_eq!(chunks[5], Bytes::from("\r\n"));

    // End chunk
    assert_eq!(chunks[6], Bytes::from("0\r\n\r\n"));
  }

  #[tokio::test]
  async fn test_chunked_transfer_empty_input() {
    let mut transformer = ChunkedTransferTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send no chunks
    drop(tx);

    // Check output - should just have end chunk
    let mut chunks = Vec::new();
    while let Some(chunk) = output.next().await {
      chunks.push(chunk);
    }

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], Bytes::from("0\r\n\r\n"));
  }

  #[tokio::test]
  async fn test_chunked_transfer_large_chunk() {
    let mut transformer = ChunkedTransferTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send a large chunk (255 bytes)
    let large_chunk = Bytes::from("x".repeat(255));
    tx.send(large_chunk.clone()).await.unwrap();
    drop(tx);

    // Check output
    let mut chunks = Vec::new();
    while let Some(chunk) = output.next().await {
      chunks.push(chunk);
    }

    // Should have: size, data, CRLF, end chunk
    assert_eq!(chunks.len(), 4);

    // Size should be "FF" in hex (255)
    assert_eq!(chunks[0], Bytes::from("FF\r\n"));

    // Data should be the large chunk
    assert_eq!(chunks[1], large_chunk);

    // CRLF
    assert_eq!(chunks[2], Bytes::from("\r\n"));

    // End chunk
    assert_eq!(chunks[3], Bytes::from("0\r\n\r\n"));
  }
}
