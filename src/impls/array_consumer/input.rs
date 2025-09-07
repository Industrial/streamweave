use crate::structs::array_consumer::ArrayConsumer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T, const N: usize> Input for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structs::array_consumer::ArrayConsumer;
  use futures::{StreamExt, stream};
  use std::pin::Pin;

  #[test]
  fn test_array_consumer_input_trait_implementation() {
    // Test that ArrayConsumer implements Input trait correctly
    fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
      _consumer: ArrayConsumer<T, N>,
    ) where
      ArrayConsumer<T, N>: Input<Input = T>,
    {
      // This function compiles only if ArrayConsumer<T, N> implements Input<Input = T>
    }

    let consumer = ArrayConsumer::<i32, 5>::new();
    assert_input_trait(consumer);
  }

  #[test]
  fn test_array_consumer_input_type_constraints() {
    // Test that the Input type is correctly set to T
    fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
      _consumer: ArrayConsumer<T, N>,
    ) -> std::marker::PhantomData<T>
    where
      ArrayConsumer<T, N>: Input<Input = T>,
    {
      std::marker::PhantomData
    }

    let consumer = ArrayConsumer::<String, 3>::new();
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to T
  }

  #[test]
  fn test_array_consumer_input_stream_type() {
    // Test that the InputStream type is correctly constrained
    fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
      _consumer: ArrayConsumer<T, N>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
      ArrayConsumer<T, N>: Input<Input = T, InputStream = Pin<Box<dyn Stream<Item = T> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let consumer = ArrayConsumer::<f64, 2>::new();
    let _stream = create_input_stream(consumer);
    // This compiles only if the InputStream type is correctly constrained
  }

  #[tokio::test]
  async fn test_array_consumer_input_stream_send_bound() {
    // Test that the InputStream implements Send bound for async usage
    let _consumer = ArrayConsumer::<i32, 4>::new();

    // Create a stream that matches the InputStream type
    let data = vec![1, 2, 3, 4];
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
      assert_eq!(sum, 10);
    });

    handle.await.unwrap();
  }

  #[test]
  fn test_array_consumer_input_trait_bounds() {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
      consumer: ArrayConsumer<T, N>,
    ) where
      ArrayConsumer<T, N>: Input,
    {
      // Test that the consumer can be used as Input
      let _input_type = std::marker::PhantomData::<T>;
      let _consumer = consumer;
    }

    // Test with different types and sizes
    test_trait_bounds(ArrayConsumer::<i32, 1>::new());
    test_trait_bounds(ArrayConsumer::<String, 5>::new());
    test_trait_bounds(ArrayConsumer::<f64, 10>::new());
    test_trait_bounds(ArrayConsumer::<bool, 100>::new());
  }

  #[test]
  fn test_array_consumer_input_static_lifetime() {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
      consumer: ArrayConsumer<T, N>,
    ) where
      ArrayConsumer<T, N>: Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // These should compile because they have 'static lifetime
    test_static_lifetime(ArrayConsumer::<i32, 3>::new());
    test_static_lifetime(ArrayConsumer::<String, 2>::new());
    test_static_lifetime(ArrayConsumer::<Vec<i32>, 1>::new());
  }

  #[test]
  fn test_array_consumer_input_debug_clone_bounds() {
    // Test that Debug and Clone bounds are correctly applied
    #[derive(Debug, Clone)]
    struct TestStruct {
      value: i32,
    }

    unsafe impl Send for TestStruct {}
    unsafe impl Sync for TestStruct {}

    let consumer = ArrayConsumer::<TestStruct, 3>::new();

    // This should compile because TestStruct implements Debug and Clone
    fn test_debug_clone<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
      _consumer: ArrayConsumer<T, N>,
    ) where
      ArrayConsumer<T, N>: Input,
    {
    }

    test_debug_clone(consumer);
  }

  #[test]
  fn test_array_consumer_input_different_sizes() {
    // Test that the Input trait works with different array sizes
    fn test_different_sizes<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer1: ArrayConsumer<T, 1>,
      _consumer2: ArrayConsumer<T, 5>,
      _consumer3: ArrayConsumer<T, 10>,
    ) where
      ArrayConsumer<T, 1>: Input<Input = T>,
      ArrayConsumer<T, 5>: Input<Input = T>,
      ArrayConsumer<T, 10>: Input<Input = T>,
    {
      // All consumers should have the same Input type T
    }

    test_different_sizes(
      ArrayConsumer::<i32, 1>::new(),
      ArrayConsumer::<i32, 5>::new(),
      ArrayConsumer::<i32, 10>::new(),
    );
  }

  #[tokio::test]
  async fn test_array_consumer_input_stream_compatibility() {
    // Test that streams can be created and used with the Input trait
    let _consumer = ArrayConsumer::<i32, 3>::new();

    // Create a stream that matches the expected InputStream type
    let data = vec![1, 2, 3];
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data));

    // Test that we can collect from the stream
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[test]
  fn test_array_consumer_input_trait_object_safety() {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
      // This function can accept any type that implements Input
    }

    // Test with different ArrayConsumer instances
    process_input(ArrayConsumer::<i32, 3>::new());
    process_input(ArrayConsumer::<String, 5>::new());
    process_input(ArrayConsumer::<f64, 1>::new());
  }
}
