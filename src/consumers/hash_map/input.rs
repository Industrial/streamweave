use super::hash_map_consumer::HashMapConsumer;
use crate::input::Input;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<K, V> Input for HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = (K, V);
  type InputStream = Pin<Box<dyn Stream<Item = (K, V)> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::{StreamExt, stream};
  use proptest::prelude::*;
  use proptest::proptest;
  use std::pin::Pin;
  use tokio::runtime::Runtime;

  async fn test_hash_map_consumer_input_stream_send_bound_async(data: Vec<(i32, String)>) {
    // Test that the InputStream implements Send bound for async usage
    let _consumer = HashMapConsumer::<i32, String>::new();

    // Create a stream that matches the InputStream type
    let stream: Pin<Box<dyn Stream<Item = (i32, String)> + Send>> =
      Box::pin(stream::iter(data.clone()));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let result: Vec<(i32, String)> = stream.collect().await;
      assert_eq!(result, data);
    });

    handle.await.unwrap();
  }

  proptest! {
    #[test]
    fn test_hash_map_consumer_input_trait_implementation(_key in any::<i32>(), _value in any::<String>()) {
      // Test that HashMapConsumer implements Input trait correctly
      fn assert_input_trait<
        K: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
        V: std::fmt::Debug + Clone + Send + Sync + 'static,
      >(
        _consumer: HashMapConsumer<K, V>,
      ) where
        HashMapConsumer<K, V>: Input<Input = (K, V)>,
      {
        // This function compiles only if HashMapConsumer<K, V> implements Input<Input = (K, V)>
      }

      let consumer = HashMapConsumer::<i32, String>::new();
      assert_input_trait(consumer);
    }

    #[test]
    fn test_hash_map_consumer_input_type_constraints(_key in any::<i32>(), _value in any::<String>()) {
      // Test that the Input type is correctly set to (K, V)
      fn get_input_type<
        K: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
        V: std::fmt::Debug + Clone + Send + Sync + 'static,
      >(
        _consumer: HashMapConsumer<K, V>,
      ) -> std::marker::PhantomData<(K, V)>
      where
        HashMapConsumer<K, V>: Input<Input = (K, V)>,
      {
        std::marker::PhantomData
      }

      let consumer = HashMapConsumer::<i32, String>::new();
      let _phantom = get_input_type(consumer);
      // This compiles only if the Input type is correctly set to (K, V)
    }

    #[test]
    fn test_hash_map_consumer_input_stream_type(_key in any::<i32>(), _value in any::<String>()) {
      // Test that the InputStream type is correctly constrained
      fn create_input_stream<
        K: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
        V: std::fmt::Debug + Clone + Send + Sync + 'static,
      >(
        _consumer: HashMapConsumer<K, V>,
      ) -> Pin<Box<dyn Stream<Item = (K, V)> + Send>>
      where
        HashMapConsumer<K, V>:
          Input<Input = (K, V), InputStream = Pin<Box<dyn Stream<Item = (K, V)> + Send>>>,
      {
        Box::pin(stream::empty())
      }

      let consumer = HashMapConsumer::<i32, String>::new();
      let _stream = create_input_stream(consumer);
      // This compiles only if the InputStream type is correctly constrained
    }

    #[test]
    fn test_hash_map_consumer_input_stream_send_bound(data in prop::collection::vec((any::<i32>(), any::<String>()), 0..20)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_hash_map_consumer_input_stream_send_bound_async(data));
    }

    #[test]
    fn test_hash_map_consumer_input_trait_bounds(_ in prop::num::u8::ANY) {
      // Test that the trait bounds are correctly applied
      fn test_trait_bounds<
        K: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
        V: std::fmt::Debug + Clone + Send + Sync + 'static,
      >(
        consumer: HashMapConsumer<K, V>,
      ) where
        HashMapConsumer<K, V>: Input,
      {
        // Test that the consumer can be used as Input
        let _consumer = consumer;
      }

      // Test with different types
      test_trait_bounds(HashMapConsumer::<i32, String>::new());
      test_trait_bounds(HashMapConsumer::<String, i32>::new());
      test_trait_bounds(HashMapConsumer::<i32, String>::new());
    }

    #[test]
    fn test_hash_map_consumer_input_static_lifetime(_ in prop::num::u8::ANY) {
      // Test that the 'static lifetime bound is correctly applied
      fn test_static_lifetime<
        K: std::fmt::Debug + Clone + Send + Sync + std::hash::Hash + Eq + 'static,
        V: std::fmt::Debug + Clone + Send + Sync + 'static,
      >(
        consumer: HashMapConsumer<K, V>,
      ) where
        HashMapConsumer<K, V>: Input,
      {
        // This function can only be called with types that have 'static lifetime
        let _consumer = consumer;
      }

      // These should compile because they have 'static lifetime
      test_static_lifetime(HashMapConsumer::<i32, String>::new());
      test_static_lifetime(HashMapConsumer::<String, i32>::new());
    }

    #[test]
    fn test_hash_map_consumer_input_stream_compatibility(data in prop::collection::vec((any::<i32>(), any::<String>()), 0..20)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(async {
        // Test that streams can be created and used with the Input trait
        let _consumer = HashMapConsumer::<i32, String>::new();

        // Create a stream that matches the expected InputStream type
        let stream: Pin<Box<dyn Stream<Item = (i32, String)> + Send>> = Box::pin(stream::iter(data.clone()));

        // Test that we can collect from the stream
        let result: Vec<(i32, String)> = stream.collect().await;
        assert_eq!(result, data);
      });
    }

    #[test]
    fn test_hash_map_consumer_input_trait_object_safety(_ in prop::num::u8::ANY) {
      // Test that the Input trait can be used with trait objects
      fn process_input<I: Input>(_input: I)
      where
        I::Input: std::fmt::Debug,
      {
        // This function can accept any type that implements Input
      }

      // Test with different HashMapConsumer instances
      process_input(HashMapConsumer::<i32, String>::new());
      process_input(HashMapConsumer::<String, i32>::new());
    }
  }
}
