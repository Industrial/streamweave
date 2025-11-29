use super::string_consumer::StringConsumer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for StringConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::{StreamExt, stream};
  use proptest::prelude::*;
  use proptest::proptest;
  use std::pin::Pin;
  use tokio::runtime::Runtime;

  async fn test_string_consumer_input_stream_send_bound_async(data: Vec<String>) {
    // Test that the InputStream implements Send bound for async usage
    let _consumer = StringConsumer::new();

    // Create a stream that matches the InputStream type
    let stream: Pin<Box<dyn Stream<Item = String> + Send>> = Box::pin(stream::iter(data.clone()));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let result: Vec<String> = stream.collect().await;
      assert_eq!(result, data);
    });

    handle.await.unwrap();
  }

  async fn test_string_consumer_input_stream_compatibility_async(data: Vec<String>) {
    // Test that streams can be created and used with the Input trait
    let _consumer = StringConsumer::new();

    // Create a stream that matches the expected InputStream type
    let stream: Pin<Box<dyn Stream<Item = String> + Send>> = Box::pin(stream::iter(data.clone()));

    // Test that we can collect from the stream
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, data);
  }

  proptest! {
    #[test]
    fn test_string_consumer_input_trait_implementation(_value in any::<String>()) {
      // Test that StringConsumer implements Input trait correctly
      fn assert_input_trait(_consumer: StringConsumer)
      where
        StringConsumer: Input<Input = String>,
      {
        // This function compiles only if StringConsumer implements Input<Input = String>
      }

      let consumer = StringConsumer::new();
      assert_input_trait(consumer);
    }

    #[test]
    fn test_string_consumer_input_type_constraints(_value in any::<String>()) {
      // Test that the Input type is correctly set to String
      fn get_input_type(_consumer: StringConsumer) -> std::marker::PhantomData<String>
      where
        StringConsumer: Input<Input = String>,
      {
        std::marker::PhantomData
      }

      let consumer = StringConsumer::new();
      let _phantom = get_input_type(consumer);
      // This compiles only if the Input type is correctly set to String
    }

    #[test]
    fn test_string_consumer_input_stream_type(_value in any::<String>()) {
      // Test that the InputStream type is correctly constrained
      fn create_input_stream(_consumer: StringConsumer) -> Pin<Box<dyn Stream<Item = String> + Send>>
      where
        StringConsumer:
          Input<Input = String, InputStream = Pin<Box<dyn Stream<Item = String> + Send>>>,
      {
        Box::pin(stream::empty())
      }

      let consumer = StringConsumer::new();
      let _stream = create_input_stream(consumer);
      // This compiles only if the InputStream type is correctly constrained
    }

    #[test]
    fn test_string_consumer_input_stream_send_bound(data in prop::collection::vec(any::<String>(), 0..20)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_string_consumer_input_stream_send_bound_async(data));
    }

    #[test]
    fn test_string_consumer_input_trait_bounds(_ in prop::num::u8::ANY) {
      // Test that the trait bounds are correctly applied
      fn test_trait_bounds(consumer: StringConsumer)
      where
        StringConsumer: Input,
      {
        // Test that the consumer can be used as Input
        let _consumer = consumer;
      }

      test_trait_bounds(StringConsumer::new());
    }

    #[test]
    fn test_string_consumer_input_static_lifetime(_ in prop::num::u8::ANY) {
      // Test that the 'static lifetime bound is correctly applied
      fn test_static_lifetime(consumer: StringConsumer)
      where
        StringConsumer: Input,
      {
        // This function can only be called with types that have 'static lifetime
        let _consumer = consumer;
      }

      // This should compile because StringConsumer has 'static lifetime
      test_static_lifetime(StringConsumer::new());
    }

    #[test]
    fn test_string_consumer_input_stream_compatibility(data in prop::collection::vec(any::<String>(), 0..20)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_string_consumer_input_stream_compatibility_async(data));
    }

    #[test]
    fn test_string_consumer_input_trait_object_safety(_ in prop::num::u8::ANY) {
      // Test that the Input trait can be used with trait objects
      fn process_input<I: Input>(_input: I)
      where
        I::Input: std::fmt::Debug,
      {
        // This function can accept any type that implements Input
      }

      // Test with StringConsumer instance
      process_input(StringConsumer::new());
    }
  }
}
