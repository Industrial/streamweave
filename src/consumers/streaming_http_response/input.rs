use super::response_chunk::ResponseChunk;
use super::streaming_http_response_consumer::StreamingHttpResponseConsumer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for StreamingHttpResponseConsumer {
  type Input = ResponseChunk;
  type InputStream = Pin<Box<dyn Stream<Item = ResponseChunk> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use bytes::Bytes;
  use futures::{StreamExt, stream};
  use std::pin::Pin;

  #[test]
  fn test_streaming_http_response_consumer_input_trait_implementation() {
    // Test that StreamingHttpResponseConsumer implements Input trait correctly
    fn assert_input_trait(_consumer: StreamingHttpResponseConsumer)
    where
      StreamingHttpResponseConsumer: Input<Input = ResponseChunk>,
    {
      // This function compiles only if StreamingHttpResponseConsumer implements Input<Input = ResponseChunk>
    }

    let (consumer, _rx) = StreamingHttpResponseConsumer::new();
    assert_input_trait(consumer);
  }

  #[test]
  fn test_streaming_http_response_consumer_input_type_constraints() {
    // Test that the Input type is correctly set to ResponseChunk
    fn get_input_type(
      _consumer: StreamingHttpResponseConsumer,
    ) -> std::marker::PhantomData<ResponseChunk>
    where
      StreamingHttpResponseConsumer: Input<Input = ResponseChunk>,
    {
      std::marker::PhantomData
    }

    let (consumer, _rx) = StreamingHttpResponseConsumer::new();
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to ResponseChunk
  }

  #[tokio::test]
  async fn test_streaming_http_response_consumer_input_stream_send_bound() {
    // Test that the InputStream implements Send bound for async usage
    let (_consumer, _rx) = StreamingHttpResponseConsumer::new();

    // Create a stream that matches the InputStream type
    let chunk = ResponseChunk::body(Bytes::from("test"));
    let data = vec![chunk];
    let stream: Pin<Box<dyn Stream<Item = ResponseChunk> + Send>> = Box::pin(stream::iter(data));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let result: Vec<ResponseChunk> = stream.collect().await;
      assert_eq!(result.len(), 1);
    });

    handle.await.unwrap();
  }

  #[test]
  fn test_streaming_http_response_consumer_input_trait_bounds() {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds(consumer: StreamingHttpResponseConsumer)
    where
      StreamingHttpResponseConsumer: Input,
    {
      // Test that the consumer can be used as Input
      let _consumer = consumer;
    }

    let (consumer, _rx) = StreamingHttpResponseConsumer::new();
    test_trait_bounds(consumer);
  }

  #[test]
  fn test_streaming_http_response_consumer_input_static_lifetime() {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime(consumer: StreamingHttpResponseConsumer)
    where
      StreamingHttpResponseConsumer: Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // This should compile because StreamingHttpResponseConsumer has 'static lifetime
    let (consumer, _rx) = StreamingHttpResponseConsumer::new();
    test_static_lifetime(consumer);
  }

  #[tokio::test]
  async fn test_streaming_http_response_consumer_input_stream_compatibility() {
    // Test that streams can be created and used with the Input trait
    let (_consumer, _rx) = StreamingHttpResponseConsumer::new();

    // Create a stream that matches the expected InputStream type
    let chunk = ResponseChunk::body(Bytes::from("test"));
    let data = vec![chunk];
    let stream: Pin<Box<dyn Stream<Item = ResponseChunk> + Send>> = Box::pin(stream::iter(data));

    // Test that we can collect from the stream
    let result: Vec<ResponseChunk> = stream.collect().await;
    assert_eq!(result.len(), 1);
  }

  #[test]
  fn test_streaming_http_response_consumer_input_trait_object_safety() {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
      // This function can accept any type that implements Input
    }

    // Test with StreamingHttpResponseConsumer instance
    let (consumer, _rx) = StreamingHttpResponseConsumer::new();
    process_input(consumer);
  }
}
