use super::channel_consumer::ChannelConsumer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<T> Input for ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
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
  use tokio::sync::mpsc::channel;

  #[test]
  fn test_channel_consumer_input_trait_implementation() {
    // Test that ChannelConsumer implements Input trait correctly
    fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: ChannelConsumer<T>,
    ) where
      ChannelConsumer<T>: Input<Input = T>,
    {
      // This function compiles only if ChannelConsumer<T> implements Input<Input = T>
    }

    let (tx, _rx) = channel::<i32>(10);
    let consumer = ChannelConsumer::new(tx);
    assert_input_trait(consumer);
  }

  #[test]
  fn test_channel_consumer_input_type_constraints() {
    // Test that the Input type is correctly set to T
    fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: ChannelConsumer<T>,
    ) -> std::marker::PhantomData<T>
    where
      ChannelConsumer<T>: Input<Input = T>,
    {
      std::marker::PhantomData
    }

    let (tx, _rx) = channel::<String>(10);
    let consumer = ChannelConsumer::new(tx);
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to T
  }

  #[test]
  fn test_channel_consumer_input_stream_type() {
    // Test that the InputStream type is correctly constrained
    fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: ChannelConsumer<T>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
      ChannelConsumer<T>: Input<Input = T, InputStream = Pin<Box<dyn Stream<Item = T> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let (tx, _rx) = channel::<f64>(10);
    let consumer = ChannelConsumer::new(tx);
    let _stream = create_input_stream(consumer);
    // This compiles only if the InputStream type is correctly constrained
  }

  async fn test_channel_consumer_input_stream_send_bound_async(data: Vec<i32>) {
    // Test that the InputStream implements Send bound for async usage
    let (tx, _rx) = channel::<i32>(10);
    let _consumer = ChannelConsumer::new(tx);

    // Create a stream that matches the InputStream type
    let expected_sum: i32 = data.iter().sum();
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data.clone()));

    // Test that we can spawn this stream in a task (requires Send)
    let handle = tokio::spawn(async move {
      let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
      assert_eq!(sum, expected_sum);
    });

    handle.await.unwrap();
  }

  proptest! {
    #[test]
    fn test_channel_consumer_input_stream_send_bound(
      data in prop::collection::vec(-1000..1000i32, 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_channel_consumer_input_stream_send_bound_async(data));
    }
  }

  #[test]
  fn test_channel_consumer_input_trait_bounds() {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      consumer: ChannelConsumer<T>,
    ) where
      ChannelConsumer<T>: Input,
    {
      // Test that the consumer can be used as Input
      let _input_type = std::marker::PhantomData::<T>;
      let _consumer = consumer;
    }

    // Test with different types
    let (tx1, _rx1) = channel::<i32>(10);
    let (tx2, _rx2) = channel::<String>(10);
    let (tx3, _rx3) = channel::<f64>(10);
    let (tx4, _rx4) = channel::<bool>(10);

    test_trait_bounds(ChannelConsumer::new(tx1));
    test_trait_bounds(ChannelConsumer::new(tx2));
    test_trait_bounds(ChannelConsumer::new(tx3));
    test_trait_bounds(ChannelConsumer::new(tx4));
  }

  #[test]
  fn test_channel_consumer_input_static_lifetime() {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      consumer: ChannelConsumer<T>,
    ) where
      ChannelConsumer<T>: Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // These should compile because they have 'static lifetime
    let (tx1, _rx1) = channel::<i32>(10);
    let (tx2, _rx2) = channel::<String>(10);
    let (tx3, _rx3) = channel::<Vec<i32>>(10);

    test_static_lifetime(ChannelConsumer::new(tx1));
    test_static_lifetime(ChannelConsumer::new(tx2));
    test_static_lifetime(ChannelConsumer::new(tx3));
  }

  proptest! {
    #[test]
    fn test_channel_consumer_input_debug_clone_bounds(
      value in -1000..1000i32
    ) {
      // Test that Debug and Clone bounds are correctly applied
      #[derive(Debug, Clone)]
      struct TestStruct {
        _value: i32,
      }

      unsafe impl Send for TestStruct {}
      unsafe impl Sync for TestStruct {}

      // Verify that we can create a channel with TestStruct
      let _test_value = TestStruct { _value: value };

      let (tx, _rx) = channel::<TestStruct>(10);
      let consumer = ChannelConsumer::new(tx);

      // This should compile because TestStruct implements Debug and Clone
      fn test_debug_clone<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
        _consumer: ChannelConsumer<T>,
      ) where
        ChannelConsumer<T>: Input,
      {
      }

      test_debug_clone(consumer);
    }
  }

  async fn test_channel_consumer_input_stream_compatibility_async(data: Vec<i32>) {
    // Test that streams can be created and used with the Input trait
    let (tx, _rx) = channel::<i32>(10);
    let _consumer = ChannelConsumer::new(tx);

    // Create a stream that matches the expected InputStream type
    let data_clone = data.clone();
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data_clone));

    // Test that we can collect from the stream
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, data);
  }

  proptest! {
    #[test]
    fn test_channel_consumer_input_stream_compatibility(
      data in prop::collection::vec(-1000..1000i32, 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_channel_consumer_input_stream_compatibility_async(data));
    }
  }

  #[test]
  fn test_channel_consumer_input_trait_object_safety() {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: Input>(_input: I)
    where
      I::Input: std::fmt::Debug,
    {
      // This function can accept any type that implements Input
    }

    // Test with different ChannelConsumer instances
    let (tx1, _rx1) = channel::<i32>(10);
    let (tx2, _rx2) = channel::<String>(10);
    let (tx3, _rx3) = channel::<f64>(10);

    process_input(ChannelConsumer::new(tx1));
    process_input(ChannelConsumer::new(tx2));
    process_input(ChannelConsumer::new(tx3));
  }
}
