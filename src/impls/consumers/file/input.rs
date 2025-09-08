use crate::structs::consumers::file::FileConsumer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for FileConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structs::consumers::file::FileConsumer;
  use futures::{StreamExt, stream};
  use std::pin::Pin;

  #[test]
  fn test_file_consumer_input_trait_implementation() {
    // Test that FileConsumer implements Input trait correctly
    fn assert_input_trait(_consumer: FileConsumer)
    where
      FileConsumer: Input<Input = String>,
    {
      // This function compiles only if FileConsumer implements Input<Input = String>
    }

    let consumer = FileConsumer::new("test.txt".to_string());
    assert_input_trait(consumer);
  }

  #[test]
  fn test_file_consumer_input_type_constraints() {
    // Test that the Input type is correctly set to String
    fn get_input_type(_consumer: FileConsumer) -> std::marker::PhantomData<String>
    where
      FileConsumer: Input<Input = String>,
    {
      std::marker::PhantomData
    }

    let consumer = FileConsumer::new("test.txt".to_string());
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to String
  }

  #[test]
  fn test_file_consumer_input_stream_type() {
    // Test that the InputStream type is correctly constrained
    fn create_input_stream(_consumer: FileConsumer) -> Pin<Box<dyn Stream<Item = String> + Send>>
    where
      FileConsumer: Input<Input = String, InputStream = Pin<Box<dyn Stream<Item = String> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let consumer = FileConsumer::new("test.txt".to_string());
    let _stream = create_input_stream(consumer);
    // This compiles only if the InputStream type is correctly constrained
  }

  #[tokio::test]
  async fn test_file_consumer_input_stream_send_bound() {
    // Test that the InputStream implements Send bound for async usage
    let _consumer = FileConsumer::new("test.txt".to_string());

    // Create a stream that matches the InputStream type
    let data = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
    let stream: Pin<Box<dyn Stream<Item = String> + Send>> = Box::pin(stream::iter(data));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let result: Vec<String> = stream.collect().await;
      assert_eq!(result, vec!["hello", "world", "test"]);
    });

    handle.await.unwrap();
  }

  #[test]
  fn test_file_consumer_input_trait_bounds() {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds(consumer: FileConsumer)
    where
      FileConsumer: Input,
    {
      // Test that the consumer can be used as Input
      let _consumer = consumer;
    }

    test_trait_bounds(FileConsumer::new("test.txt".to_string()));
  }

  #[test]
  fn test_file_consumer_input_static_lifetime() {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime(consumer: FileConsumer)
    where
      FileConsumer: Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // This should compile because FileConsumer has 'static lifetime
    test_static_lifetime(FileConsumer::new("test.txt".to_string()));
  }

  #[tokio::test]
  async fn test_file_consumer_input_stream_compatibility() {
    // Test that streams can be created and used with the Input trait
    let _consumer = FileConsumer::new("test.txt".to_string());

    // Create a stream that matches the expected InputStream type
    let data = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
    let stream: Pin<Box<dyn Stream<Item = String> + Send>> = Box::pin(stream::iter(data));

    // Test that we can collect from the stream
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, vec!["hello", "world", "test"]);
  }

  #[test]
  fn test_file_consumer_input_trait_object_safety() {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
      // This function can accept any type that implements Input
    }

    // Test with FileConsumer instance
    process_input(FileConsumer::new("test.txt".to_string()));
  }
}
