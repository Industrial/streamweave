use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{input::Input, output::Output};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct TransformerConfig<T: std::fmt::Debug + Clone + Send + Sync> {
  pub error_strategy: ErrorStrategy<T>,
  pub name: Option<String>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Default for TransformerConfig<T> {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: None,
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> TransformerConfig<T> {
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.name = Some(name);
    self
  }

  pub fn error_strategy(&self) -> ErrorStrategy<T> {
    self.error_strategy.clone()
  }

  pub fn name(&self) -> Option<String> {
    self.name.clone()
  }
}

#[async_trait]
pub trait Transformer: Input + Output
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream;

  fn with_config(&self, config: TransformerConfig<Self::Input>) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  fn set_config(&mut self, config: TransformerConfig<Self::Input>) {
    self.set_config_impl(config);
  }

  fn config(&self) -> &TransformerConfig<Self::Input> {
    self.get_config_impl()
  }

  fn config_mut(&mut self) -> &mut TransformerConfig<Self::Input> {
    self.get_config_mut_impl()
  }

  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    let config = self.get_config_impl().clone();
    self.set_config(TransformerConfig {
      error_strategy: config.error_strategy,
      name: Some(name),
    });
    self
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  // These methods need to be implemented by each transformer
  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>);
  fn get_config_impl(&self) -> &TransformerConfig<Self::Input>;
  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::{ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError};
  use futures::StreamExt;
  use std::fmt;
  use std::pin::Pin;
  use std::sync::{Arc, Mutex};
  use tokio_stream::Stream;

  // Test error type
  #[derive(Debug)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  // Test transformer that doubles the input
  #[derive(Clone)]
  struct TestTransformer<T: std::fmt::Debug + Clone + Send + Sync> {
    config: TransformerConfig<T>,
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync> TestTransformer<T> {
    fn new() -> Self {
      Self {
        config: TransformerConfig::default(),
      }
    }
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TestTransformer<T> {
    type Input = T;
    type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
  }

  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TestTransformer<T> {
    type Output = T;
    type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
  }

  #[async_trait]
  impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TestTransformer<T> {
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      Box::pin(input)
    }

    fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
      &self.config
    }

    fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
      &mut self.config
    }
  }

  #[tokio::test]
  async fn test_transformer() {
    let mut transformer = TestTransformer::<i32>::new();
    let input = futures::stream::iter(vec![1, 2, 3]);
    let output = transformer.transform(Box::pin(input));
    let result: Vec<i32> = output.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[test]
  fn test_transformer_config() {
    let mut transformer = TestTransformer::<i32>::new()
      .with_name("test_transformer".to_string())
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

    assert_eq!(
      transformer.config().name(),
      Some("test_transformer".to_string())
    );
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_transformer_error_handling() {
    let mut transformer = TestTransformer::<i32>::new()
      .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: transformer.component_info().name,
        component_type: transformer.component_info().type_name,
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));
  }
}
