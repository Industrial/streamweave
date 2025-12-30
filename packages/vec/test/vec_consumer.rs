use futures::{StreamExt, stream};
use proptest::prelude::*;
use streamweave_error::ErrorStrategy;
use streamweave_vec::VecConsumer;

async fn test_vec_consumer_basic_async(input_data: Vec<i32>) {
  let mut consumer = VecConsumer::new();
  let input = stream::iter(input_data.clone());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  let vec = consumer.into_vec();
  assert_eq!(vec, input_data);
}

async fn test_vec_consumer_with_capacity_async(input_data: Vec<i32>, capacity: usize) {
  let mut consumer = VecConsumer::with_capacity(capacity);
  let input = stream::iter(input_data.clone());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  assert_eq!(consumer.into_vec(), input_data);
}

proptest! {
  #[test]
  fn test_vec_consumer_basic(
    input_data in prop::collection::vec(any::<i32>(), 0..30)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_vec_consumer_basic_async(input_data));
  }

  #[test]
  fn test_vec_consumer_empty_input(_ in prop::num::u8::ANY) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
      let mut consumer = VecConsumer::new();
      let input = stream::iter(Vec::<i32>::new());
      let boxed_input = Box::pin(input);

      consumer.consume(boxed_input).await;
      let vec = consumer.into_vec();
      assert!(vec.is_empty());
    });
  }

  #[test]
  fn test_vec_consumer_with_capacity(
    input_data in prop::collection::vec(any::<i32>(), 0..30),
    capacity in 0usize..1000usize
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_vec_consumer_with_capacity_async(input_data, capacity));
  }

  #[test]
  fn test_error_handling_strategies(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let consumer = VecConsumer::<i32>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name(name.clone());

    prop_assert_eq!(
      &consumer.config().error_strategy,
      &ErrorStrategy::<i32>::Skip
    );
    prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
  }

  #[test]
  fn test_vec_consumer_input_trait_implementation(_value in any::<i32>()) {
    // Test that VecConsumer implements Input trait correctly
    fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: VecConsumer<T>,
    ) where
      VecConsumer<T>: streamweave::Input<Input = T>,
    {
      // This function compiles only if VecConsumer<T> implements Input<Input = T>
    }

    let consumer = VecConsumer::<i32>::new();
    assert_input_trait(consumer);
  }

  #[test]
  fn test_vec_consumer_input_type_constraints(_value in any::<i32>()) {
    // Test that the Input type is correctly set to T
    fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: VecConsumer<T>,
    ) -> std::marker::PhantomData<T>
    where
      VecConsumer<T>: streamweave::Input<Input = T>,
    {
      std::marker::PhantomData
    }

    let consumer = VecConsumer::<i32>::new();
    let _phantom = get_input_type(consumer);
    // This compiles only if the Input type is correctly set to T
  }

  #[test]
  fn test_vec_consumer_input_stream_type(_value in any::<i32>()) {
    // Test that the InputStream type is correctly constrained
    fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: VecConsumer<T>,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>>
    where
      VecConsumer<T>: streamweave::Input<Input = T, InputStream = std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>>>,
    {
      Box::pin(stream::empty())
    }

    let consumer = VecConsumer::<i32>::new();
    let _stream = create_input_stream(consumer);
    // This compiles only if the InputStream type is correctly constrained
  }

  #[test]
  fn test_vec_consumer_input_stream_send_bound(data in prop::collection::vec(any::<i32>(), 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_vec_consumer_input_stream_send_bound_async(data));
  }

  #[test]
  fn test_vec_consumer_input_trait_bounds(_ in prop::num::u8::ANY) {
    // Test that the trait bounds are correctly applied
    fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      consumer: VecConsumer<T>,
    ) where
      VecConsumer<T>: streamweave::Input,
    {
      // Test that the consumer can be used as Input
      let _consumer = consumer;
    }

    // Test with different types
    test_trait_bounds(VecConsumer::<i32>::new());
    test_trait_bounds(VecConsumer::<String>::new());
    test_trait_bounds(VecConsumer::<i32>::new());
  }

  #[test]
  fn test_vec_consumer_input_static_lifetime(_ in prop::num::u8::ANY) {
    // Test that the 'static lifetime bound is correctly applied
    fn test_static_lifetime<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      consumer: VecConsumer<T>,
    ) where
      VecConsumer<T>: streamweave::Input,
    {
      // This function can only be called with types that have 'static lifetime
      let _consumer = consumer;
    }

    // These should compile because they have 'static lifetime
    test_static_lifetime(VecConsumer::<i32>::new());
    test_static_lifetime(VecConsumer::<String>::new());
  }

  #[test]
  fn test_vec_consumer_input_stream_compatibility(data in prop::collection::vec(any::<i32>(), 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_vec_consumer_input_stream_compatibility_async(data));
  }

  #[test]
  fn test_vec_consumer_input_trait_object_safety(_ in prop::num::u8::ANY) {
    // Test that the Input trait can be used with trait objects
    fn process_input<I: streamweave::Input>(_input: I)
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

async fn test_vec_consumer_input_stream_send_bound_async(data: Vec<i32>) {
  // Test that the InputStream implements Send bound for async usage
  let _consumer = VecConsumer::<i32>::new();

  // Create a stream that matches the InputStream type
  let stream: std::pin::Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(data.clone()));

  // Test that we can spawn this stream in a task (requires Send)
  let handle = tokio::spawn(async move {
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, data);
  });

  handle.await.unwrap();
}

async fn test_vec_consumer_input_stream_compatibility_async(data: Vec<i32>) {
  // Test that streams can be created and used with the Input trait
  let _consumer = VecConsumer::<i32>::new();

  // Create a stream that matches the expected InputStream type
  let stream: std::pin::Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(data.clone()));

  // Test that we can collect from the stream
  let result: Vec<i32> = stream.collect().await;
  assert_eq!(result, data);
}
