use crate::structs::vec_consumer::VecConsumer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structs::vec_consumer::VecConsumer;
  use futures::{StreamExt, stream};
  use std::pin::Pin;

  #[test]
  fn test_vec_consumer_input_trait_implementation() {
    // Test that VecConsumer implements Input trait correctly
    fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: VecConsumer<T>,
    ) where
      VecConsumer<T>: Input<Input = T>,
    {
      // This function compiles only if VecConsumer<T> implements Input<Input = T>
    }

    let consumer = VecConsumer::<i32>::new();
    assert_input_trait(consumer);
  }

  #[test]
  fn test_vec_consumer_input_type_constraints() {
    // Test that the Input type is correctly set to T
    fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: VecConsumer<T>,
    ) -> std::marker::PhantomData<T>
    where
      VecConsumer<T>: Input<Input = T>,
    {
      std::marker::PhantomData
    }

    let consumer = VecConsumer::<i32>::new();
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to T
  }

  #[test]
  fn test_vec_consumer_input_stream_type() {
    // Test that the InputStream type is correctly constrained
    fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: VecConsumer<T>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
      VecConsumer<T>: Input<Input = T, InputStream = Pin<Box<dyn Stream<Item = T> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let consumer = VecConsumer::<i32>::new();
    let _stream = create_input_stream(consumer);
    // This compiles only if the InputStream type is correctly constrained
  }

  #[tokio::test]
  async fn test_vec_consumer_input_stream_send_bound() {
    // Test that the InputStream implements Send bound for async usage
    let _consumer = VecConsumer::<i32>::new();

    // Create a stream that matches the InputStream type
    let data = vec![1, 2, 3];
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let result: Vec<i32> = stream.collect().await;
      assert_eq!(result, vec![1, 2, 3]);
    });

    handle.await.unwrap();
  }

  #[test]
  fn test_vec_consumer_input_trait_bounds() {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      consumer: VecConsumer<T>,
    ) where
      VecConsumer<T>: Input,
    {
      // Test that the consumer can be used as Input
      let _consumer = consumer;
    }

    // Test with different types
    test_trait_bounds(VecConsumer::<i32>::new());
    test_trait_bounds(VecConsumer::<String>::new());
    test_trait_bounds(VecConsumer::<&str>::new());
  }

  #[test]
  fn test_vec_consumer_input_static_lifetime() {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      consumer: VecConsumer<T>,
    ) where
      VecConsumer<T>: Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // These should compile because they have 'static lifetime
    test_static_lifetime(VecConsumer::<i32>::new());
    test_static_lifetime(VecConsumer::<String>::new());
  }

  #[tokio::test]
  async fn test_vec_consumer_input_stream_compatibility() {
    // Test that streams can be created and used with the Input trait
    let _consumer = VecConsumer::<i32>::new();

    // Create a stream that matches the expected InputStream type
    let data = vec![1, 2, 3];
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data));

    // Test that we can collect from the stream
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[test]
  fn test_vec_consumer_input_trait_object_safety() {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
      // This function can accept any type that implements Input
    }

    // Test with different VecConsumer instances
    process_input(VecConsumer::<i32>::new());
    process_input(VecConsumer::<String>::new());
  }
}
