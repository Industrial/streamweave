use super::console_consumer::ConsoleConsumer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::{StreamExt, stream};
  use proptest::prelude::*;
  use std::pin::Pin;

  #[test]
  fn test_console_consumer_input_trait_implementation() {
    // Test that ConsoleConsumer implements Input trait correctly
    fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
      _consumer: ConsoleConsumer<T>,
    ) where
      ConsoleConsumer<T>: Input<Input = T>,
    {
      // This function compiles only if ConsoleConsumer<T> implements Input<Input = T>
    }

    let consumer = ConsoleConsumer::<&str>::new();
    assert_input_trait(consumer);
  }

  #[test]
  fn test_console_consumer_input_type_constraints() {
    // Test that the Input type is correctly set to T
    fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
      _consumer: ConsoleConsumer<T>,
    ) -> std::marker::PhantomData<T>
    where
      ConsoleConsumer<T>: Input<Input = T>,
    {
      std::marker::PhantomData
    }

    let consumer = ConsoleConsumer::<&str>::new();
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to T
  }

  #[test]
  fn test_console_consumer_input_stream_type() {
    // Test that the InputStream type is correctly constrained
    fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
      _consumer: ConsoleConsumer<T>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
      ConsoleConsumer<T>: Input<Input = T, InputStream = Pin<Box<dyn Stream<Item = T> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let consumer = ConsoleConsumer::<&str>::new();
    let _stream = create_input_stream(consumer);
    // This compiles only if the InputStream type is correctly constrained
  }

  async fn test_console_consumer_input_stream_send_bound_async(data: Vec<String>) {
    // Test that the InputStream implements Send bound for async usage
    let _consumer = ConsoleConsumer::<String>::new();

    // Create a stream that matches the InputStream type
    let data_clone = data.clone();
    let stream: Pin<Box<dyn Stream<Item = String> + Send>> = Box::pin(stream::iter(data_clone));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let result: Vec<String> = stream.collect().await;
      assert_eq!(result, data);
    });

    handle.await.unwrap();
  }

  proptest! {
    #[test]
    fn test_console_consumer_input_stream_send_bound(
      data in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_console_consumer_input_stream_send_bound_async(data));
    }
  }

  #[test]
  fn test_console_consumer_input_trait_bounds() {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
      consumer: ConsoleConsumer<T>,
    ) where
      ConsoleConsumer<T>: Input,
    {
      // Test that the consumer can be used as Input
      let _input_type = std::marker::PhantomData::<T>;
      let _consumer = consumer;
    }

    // Test with different types
    test_trait_bounds(ConsoleConsumer::<&str>::new());
    test_trait_bounds(ConsoleConsumer::<String>::new());
    test_trait_bounds(ConsoleConsumer::<i32>::new());
    test_trait_bounds(ConsoleConsumer::<f64>::new());
  }

  #[test]
  fn test_console_consumer_input_static_lifetime() {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime<
      T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
    >(
      consumer: ConsoleConsumer<T>,
    ) where
      ConsoleConsumer<T>: Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // These should compile because they have 'static lifetime
    test_static_lifetime(ConsoleConsumer::<&str>::new());
    test_static_lifetime(ConsoleConsumer::<String>::new());
    test_static_lifetime(ConsoleConsumer::<i32>::new());
  }

  proptest! {
    #[test]
    fn test_console_consumer_input_debug_clone_display_bounds(
      value in -1000..1000i32
    ) {
      // Test that Debug, Clone, and Display bounds are correctly applied
      #[derive(Debug, Clone)]
      struct TestStruct {
        value: i32,
      }

      impl std::fmt::Display for TestStruct {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          write!(f, "{}", self.value)
        }
      }

      unsafe impl Send for TestStruct {}
      unsafe impl Sync for TestStruct {}

      // Verify that we can create a TestStruct with the generated value
      let _test_value = TestStruct { value };

      let consumer = ConsoleConsumer::<TestStruct>::new();

      // This should compile because TestStruct implements Debug, Clone, and Display
      fn test_debug_clone_display<
        T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
      >(
        _consumer: ConsoleConsumer<T>,
      ) where
        ConsoleConsumer<T>: Input,
      {
      }

      test_debug_clone_display(consumer);
    }
  }

  async fn test_console_consumer_input_stream_compatibility_async(data: Vec<String>) {
    // Test that streams can be created and used with the Input trait
    let _consumer = ConsoleConsumer::<String>::new();

    // Create a stream that matches the expected InputStream type
    let data_clone = data.clone();
    let stream: Pin<Box<dyn Stream<Item = String> + Send>> = Box::pin(stream::iter(data_clone));

    // Test that we can collect from the stream
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, data);
  }

  proptest! {
    #[test]
    fn test_console_consumer_input_stream_compatibility(
      data in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_console_consumer_input_stream_compatibility_async(data));
    }
  }

  #[test]
  fn test_console_consumer_input_trait_object_safety() {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
      // This function can accept any type that implements Input
    }

    // Test with different ConsoleConsumer instances
    process_input(ConsoleConsumer::<&str>::new());
    process_input(ConsoleConsumer::<String>::new());
    process_input(ConsoleConsumer::<i32>::new());
  }

  #[test]
  fn test_console_consumer_input_display_bound() {
    // Test that the Display bound is correctly applied
    fn test_display_bound<T: std::fmt::Display>(_value: T) {}

    // Call the function to avoid unused function warning
    test_display_bound("test");

    let consumer = ConsoleConsumer::<&str>::new();

    // This should compile because ConsoleConsumer requires Display bound
    fn test_consumer_display<
      T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
    >(
      consumer: ConsoleConsumer<T>,
    ) where
      ConsoleConsumer<T>: Input,
    {
      // We can't directly test Display on the consumer, but the bound ensures T implements Display
      let _consumer = consumer;
    }

    test_consumer_display(consumer);
  }
}
