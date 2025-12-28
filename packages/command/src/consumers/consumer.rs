use super::command_consumer::CommandConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T> Consumer for CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      if let Some(cmd) = &mut self.command {
        let output = cmd.arg(value.to_string()).output().await;
        if let Err(e) = output {
          eprintln!("Failed to execute command: {}", e);
        }
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

  async fn test_command_consumer_basic_async(input: Vec<String>) {
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_command_consumer_basic(
      input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..20)
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_command_consumer_basic_async(input));
    }
  }

  async fn test_command_consumer_empty_input_async() {
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
    let input = stream::iter(Vec::<String>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[test]
  fn test_command_consumer_empty_input() {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(test_command_consumer_empty_input_async());
  }

  async fn test_error_handling_strategies_async(name: String) {
    let consumer = CommandConsumer::new("echo".to_string(), vec![])
      .with_error_strategy(ErrorStrategy::<String>::Skip)
      .with_name(name.clone());

    assert_eq!(
      consumer.config().error_strategy,
      ErrorStrategy::<String>::Skip
    );
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

  async fn test_error_handling_during_consumption_async(error_msg: String, item: String) {
    let consumer = CommandConsumer::new("echo".to_string(), vec![])
      .with_error_strategy(ErrorStrategy::<String>::Skip)
      .with_name("test_consumer".to_string());

    // Test that Skip strategy allows consumption to continue
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::other(error_msg.clone())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(item.clone()),
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
    let consumer = consumer.with_error_strategy(ErrorStrategy::<String>::Stop);
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
      item in prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap()
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_error_handling_during_consumption_async(error_msg, item));
    }
  }

  async fn test_component_info_async() {
    let consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
    let info = consumer.component_info();
    assert_eq!(info.name, "");
    assert_eq!(
      info.type_name,
      "streamweave_command::consumers::command_consumer::CommandConsumer<alloc::string::String>"
    );
  }

  #[test]
  fn test_component_info() {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(test_component_info_async());
  }

  async fn test_error_context_creation_async(item: String) {
    let consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
    let context = consumer.create_error_context(Some(item.clone()));
    assert_eq!(context.component_name, "");
    assert_eq!(
      context.component_type,
      "streamweave_command::consumers::command_consumer::CommandConsumer<alloc::string::String>"
    );
    assert_eq!(context.item, Some(item));
  }

  proptest! {
    #[test]
    fn test_error_context_creation(
      item in prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap()
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_error_context_creation_async(item));
    }
  }

  async fn test_command_with_arguments_async(input: Vec<String>, args: Vec<String>) {
    let mut consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), args);
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_command_with_arguments(
      input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..10),
      args in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9-]+").unwrap(), 0..5)
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_command_with_arguments_async(input, args));
    }
  }

  async fn test_command_execution_failure_async(input: Vec<String>) {
    let mut consumer: CommandConsumer<String> =
      CommandConsumer::new("nonexistent_command".to_string(), vec![]);
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    // Should not panic when command execution fails
    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_command_execution_failure(
      input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..10)
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_command_execution_failure_async(input));
    }
  }

  async fn test_command_output_handling_async(input: Vec<String>) {
    let mut consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_command_output_handling(
      input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..10)
    ) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_command_output_handling_async(input));
    }
  }
}
