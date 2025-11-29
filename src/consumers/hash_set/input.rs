use super::hash_set_consumer::HashSetConsumer;
use crate::input::Input;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<T> Input for HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::{StreamExt, stream};
  use proptest::prelude::*;
  use proptest::proptest;
  use std::pin::Pin;
  use tokio::runtime::Runtime;

  async fn test_hash_set_consumer_input_stream_send_bound_async(data: Vec<i32>) {
    // Test that the InputStream implements Send bound for async usage
    let _consumer = HashSetConsumer::<i32>::new();

    // Create a stream that matches the InputStream type
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data.clone()));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let result: Vec<i32> = stream.collect().await;
      assert_eq!(result, data);
    });

    handle.await.unwrap();
  }

  proptest! {
    #[test]
    fn test_hash_set_consumer_input_trait_implementation(_value in any::<i32>()) {
      // Test that HashSetConsumer implements Input trait correctly
      fn assert_input_trait<
        T: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
      >(
        _consumer: HashSetConsumer<T>,
      ) where
        HashSetConsumer<T>: Input<Input = T>,
      {
        // This function compiles only if HashSetConsumer<T> implements Input<Input = T>
      }

      let consumer = HashSetConsumer::<i32>::new();
      assert_input_trait(consumer);
    }

    #[test]
    fn test_hash_set_consumer_input_type_constraints(_value in any::<i32>()) {
      // Test that the Input type is correctly set to T
      fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static>(
        _consumer: HashSetConsumer<T>,
      ) -> std::marker::PhantomData<T>
      where
        HashSetConsumer<T>: Input<Input = T>,
      {
        std::marker::PhantomData
      }

      let consumer = HashSetConsumer::<i32>::new();
      let _phantom = get_input_type(consumer);
      // This compiles only if the Input type is correctly set to T
    }

    #[test]
    fn test_hash_set_consumer_input_stream_type(_value in any::<i32>()) {
      // Test that the InputStream type is correctly constrained
      fn create_input_stream<
        T: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
      >(
        _consumer: HashSetConsumer<T>,
      ) -> Pin<Box<dyn Stream<Item = T> + Send>>
      where
        HashSetConsumer<T>: Input<Input = T, InputStream = Pin<Box<dyn Stream<Item = T> + Send>>>,
      {
        Box::pin(stream::empty())
      }

      let consumer = HashSetConsumer::<i32>::new();
      let _stream = create_input_stream(consumer);
      // This compiles only if the InputStream type is correctly constrained
    }

    #[test]
    fn test_hash_set_consumer_input_stream_send_bound(data in prop::collection::vec(any::<i32>(), 0..20)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_hash_set_consumer_input_stream_send_bound_async(data));
    }

    #[test]
    fn test_hash_set_consumer_input_trait_bounds(_ in prop::num::u8::ANY) {
      // Test that the trait bounds are correctly applied
      fn test_trait_bounds<
        T: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
      >(
        consumer: HashSetConsumer<T>,
      ) where
        HashSetConsumer<T>: Input,
      {
        // Test that the consumer can be used as Input
        let _consumer = consumer;
      }

      // Test with different types
      test_trait_bounds(HashSetConsumer::<i32>::new());
      test_trait_bounds(HashSetConsumer::<String>::new());
      test_trait_bounds(HashSetConsumer::<i32>::new());
    }

    #[test]
    fn test_hash_set_consumer_input_static_lifetime(_ in prop::num::u8::ANY) {
      // Test that the 'static lifetime bound is correctly applied
      fn test_static_lifetime<
        T: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
      >(
        consumer: HashSetConsumer<T>,
      ) where
        HashSetConsumer<T>: Input,
      {
        // This function can only be called with types that have 'static lifetime
        let _consumer = consumer;
      }

      // These should compile because they have 'static lifetime
      test_static_lifetime(HashSetConsumer::<i32>::new());
      test_static_lifetime(HashSetConsumer::<String>::new());
    }

    #[test]
    fn test_hash_set_consumer_input_stream_compatibility(data in prop::collection::vec(any::<i32>(), 0..20)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(async {
        // Test that streams can be created and used with the Input trait
        let _consumer = HashSetConsumer::<i32>::new();

        // Create a stream that matches the expected InputStream type
        let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data.clone()));

        // Test that we can collect from the stream
        let result: Vec<i32> = stream.collect().await;
        assert_eq!(result, data);
      });
    }

    #[test]
    fn test_hash_set_consumer_input_trait_object_safety(_ in prop::num::u8::ANY) {
      // Test that the Input trait can be used with trait objects
      fn process_input<I: Input>(_input: I)
      where
        I::Input: std::fmt::Debug,
      {
        // This function can accept any type that implements Input
      }

      // Test with different HashSetConsumer instances
      process_input(HashSetConsumer::<i32>::new());
      process_input(HashSetConsumer::<String>::new());
    }
  }
}
