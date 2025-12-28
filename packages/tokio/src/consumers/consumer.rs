use super::channel_consumer::ChannelConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T> Consumer for ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, input: Self::InputStream) -> () {
    let mut stream = input;
    while let Some(value) = stream.next().await {
      if let Some(sender) = &self.channel
        && let Err(e) = sender.send(value).await
      {
        eprintln!("Failed to send value to channel: {}", e);
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use proptest::prelude::*;
  use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use tokio::sync::mpsc::{Receiver, Sender, channel};

  async fn test_channel_consumer_basic_async(input: Vec<i32>) {
    let (tx, mut rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    // Spawn the consumer in a separate task
    let handle = tokio::spawn(async move {
      consumer.consume(boxed_input).await;
    });

    // Wait for all items to be received
    for expected_item in input {
      assert_eq!(rx.recv().await, Some(expected_item));
    }
    assert_eq!(rx.recv().await, None);

    // Wait for the consumer task to complete
    handle.await.unwrap();
  }

  proptest! {
    #[test]
    fn test_channel_consumer_basic(
      input in prop::collection::vec(-1000..1000i32, 0..100)
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_channel_consumer_basic_async(input));
    }
  }

  async fn test_channel_consumer_empty_input_async() {
    let (tx, mut rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    // Spawn the consumer in a separate task
    let handle = tokio::spawn(async move {
      consumer.consume(boxed_input).await;
    });

    // Verify no items are received
    assert_eq!(rx.recv().await, None);

    // Wait for the consumer task to complete
    handle.await.unwrap();
  }

  #[test]
  fn test_channel_consumer_empty_input() {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(test_channel_consumer_empty_input_async());
  }

  async fn test_error_handling_strategies_async(name: String) {
    let (tx, _rx) = channel(10);
    let consumer = ChannelConsumer::new(tx)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name(name.clone());

    assert_eq!(consumer.config().error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(consumer.config().name, name);
  }

  proptest! {
    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_error_handling_strategies_async(name));
    }
  }

  async fn test_error_handling_during_consumption_async(error_msg: String, item: i32) {
    let (tx, _rx) = channel(10);
    let consumer = ChannelConsumer::new(tx)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    // Test that Skip strategy allows consumption to continue
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::other(error_msg.clone())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(item),
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    });
    assert_eq!(action, ErrorAction::Skip);

    // Test that Stop strategy halts consumption
    let consumer = consumer.with_error_strategy(ErrorStrategy::<i32>::Stop);
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::other(error_msg)),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(item),
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    });
    assert_eq!(action, ErrorAction::Stop);
  }

  proptest! {
    #[test]
    fn test_error_handling_during_consumption(
      error_msg in prop::string::string_regex(".+").unwrap(),
      item in -1000..1000i32
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_error_handling_during_consumption_async(error_msg, item));
    }
  }

  async fn test_component_info_async(name: String) {
    let (tx, _rx): (Sender<i32>, Receiver<i32>) = channel(10);
    let consumer = ChannelConsumer::new(tx).with_name(name.clone());

    let info = consumer.component_info();
    assert_eq!(info.name, name);
    assert_eq!(
      info.type_name,
      "streamweave_tokio::consumers::channel_consumer::ChannelConsumer<i32>"
    );
  }

  proptest! {
    #[test]
    fn test_component_info(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_component_info_async(name));
    }
  }

  async fn test_error_context_creation_async(name: String, item: i32) {
    let (tx, _rx): (Sender<i32>, Receiver<i32>) = channel(10);
    let consumer = ChannelConsumer::new(tx).with_name(name.clone());

    let context = consumer.create_error_context(Some(item));
    assert_eq!(context.component_name, name);
    assert_eq!(
      context.component_type,
      "streamweave_tokio::consumers::channel_consumer::ChannelConsumer<i32>"
    );
    assert_eq!(context.item, Some(item));
  }

  proptest! {
    #[test]
    fn test_error_context_creation(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      item in -1000..1000i32
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_error_context_creation_async(name, item));
    }
  }

  async fn test_channel_capacity_async(input: Vec<i32>, capacity: usize) {
    let (tx, mut rx): (Sender<i32>, Receiver<i32>) = channel(capacity);
    let mut consumer = ChannelConsumer::new(tx);

    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    // Spawn the consumer in a separate task
    let handle = tokio::spawn(async move {
      consumer.consume(boxed_input).await;
    });

    // Should receive all items despite capacity limit
    for expected_item in input {
      assert_eq!(rx.recv().await, Some(expected_item));
    }
    assert_eq!(rx.recv().await, None);

    // Wait for the consumer task to complete
    handle.await.unwrap();
  }

  proptest! {
    #[test]
    fn test_channel_capacity(
      input in prop::collection::vec(-1000..1000i32, 1..50),
      capacity in 1..10usize
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_channel_capacity_async(input, capacity));
    }
  }

  async fn test_dropped_channel_async(input: Vec<i32>) {
    let (tx, rx): (Sender<i32>, Receiver<i32>) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    // Drop the receiver
    drop(rx);

    let input_stream = stream::iter(input);
    let boxed_input = Box::pin(input_stream);

    // Spawn the consumer in a separate task
    let handle = tokio::spawn(async move {
      consumer.consume(boxed_input).await;
    });

    // Should not panic when sending to dropped channel
    handle.await.unwrap();
  }

  proptest! {
    #[test]
    fn test_dropped_channel(
      input in prop::collection::vec(-1000..1000i32, 0..50)
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_dropped_channel_async(input));
    }
  }
}
