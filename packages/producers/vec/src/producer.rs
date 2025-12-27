use super::vec_producer::VecProducer;
use futures::stream;
use streamweave_core::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Producer for VecProducer<T> {
  type OutputPorts = (T,);

  fn produce(&mut self) -> Self::OutputStream {
    let producer_name = self.config.name().unwrap_or("vec_producer".to_string());
    println!("📤 [{}] Producing {} items", producer_name, self.data.len());
    let stream = stream::iter(self.data.clone());
    Box::pin(stream)
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "vec_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "vec_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_vec_producer() {
    let data = vec!["test1".to_string(), "test2".to_string()];
    let mut producer = VecProducer::new(data.clone());
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, data);
  }

  #[tokio::test]
  async fn test_vec_producer_empty() {
    let mut producer = VecProducer::<String>::new(vec![]);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, Vec::<String>::new());
  }

  #[tokio::test]
  async fn test_vec_producer_multiple_calls() {
    let data = vec!["test1".to_string(), "test2".to_string()];
    let mut producer = VecProducer::new(data.clone());

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.collect().await;
    assert_eq!(result1, data);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.collect().await;
    assert_eq!(result2, data);
  }

  #[tokio::test]
  async fn test_vec_producer_clone_independence() {
    let mut producer1 = VecProducer::new(vec![1, 2, 3]);
    let mut producer2 = VecProducer::new(vec![4, 5, 6]);

    let stream1 = producer1.produce();
    let stream2 = producer2.produce();

    let result1: Vec<i32> = stream1.collect().await;
    let result2: Vec<i32> = stream2.collect().await;

    assert_eq!(result1, vec![1, 2, 3]);
    assert_eq!(result2, vec![4, 5, 6]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let producer = VecProducer::new(vec![1, 2, 3])
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "VecProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "VecProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }

  #[test]
  fn test_handle_error_stop() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "VecProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "VecProducer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(producer.handle_error(&error), ErrorAction::Stop));
  }

  #[test]
  fn test_handle_error_retry() {
    let producer = VecProducer::new(vec![1, 2, 3]).with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "VecProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "VecProducer".to_string(),
      },
      retries: 1,
    };

    assert!(matches!(producer.handle_error(&error), ErrorAction::Retry));
  }

  #[test]
  fn test_handle_error_retry_exhausted() {
    let producer = VecProducer::new(vec![1, 2, 3]).with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "VecProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "VecProducer".to_string(),
      },
      retries: 3,
    };

    assert!(matches!(producer.handle_error(&error), ErrorAction::Stop));
  }

  #[test]
  fn test_handle_error_custom() {
    let producer = VecProducer::new(vec![1, 2, 3]).with_error_strategy(ErrorStrategy::new_custom(
      |error: &StreamError<i32>| {
        if error.retries < 2 {
          ErrorAction::Retry
        } else {
          ErrorAction::Skip
        }
      },
    ));

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "VecProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "VecProducer".to_string(),
      },
      retries: 1,
    };

    assert!(matches!(producer.handle_error(&error), ErrorAction::Retry));

    let error_exhausted = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "VecProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "VecProducer".to_string(),
      },
      retries: 2,
    };

    assert!(matches!(
      producer.handle_error(&error_exhausted),
      ErrorAction::Skip
    ));
  }

  #[test]
  fn test_set_config_impl() {
    let mut producer = VecProducer::new(vec![1, 2, 3]);
    let new_config = ProducerConfig {
      name: Some("new_name".to_string()),
      error_strategy: ErrorStrategy::Skip,
    };

    producer.set_config_impl(new_config.clone());

    let retrieved_config = producer.get_config_impl();
    assert_eq!(retrieved_config.name(), Some("new_name".to_string()));
    assert!(matches!(
      retrieved_config.error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_get_config_mut_impl() {
    let mut producer = VecProducer::new(vec![1, 2, 3]);
    let config_mut = producer.get_config_mut_impl();
    config_mut.name = Some("mutated_name".to_string());
    config_mut.error_strategy = ErrorStrategy::Retry(5);

    let config = producer.get_config_impl();
    assert_eq!(config.name(), Some("mutated_name".to_string()));
    assert!(matches!(config.error_strategy(), ErrorStrategy::Retry(5)));
  }

  #[test]
  fn test_create_error_context_with_item() {
    let producer = VecProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());
    let context = producer.create_error_context(Some(42));

    assert_eq!(context.component_name, "test_producer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<VecProducer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_create_error_context_with_none() {
    let producer = VecProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());
    let context = producer.create_error_context(None);

    assert_eq!(context.component_name, "test_producer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<VecProducer<i32>>()
    );
    assert_eq!(context.item, None);
  }

  #[test]
  fn test_create_error_context_default_name() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let context = producer.create_error_context(Some(42));

    assert_eq!(context.component_name, "vec_producer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<VecProducer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_component_info() {
    let producer = VecProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());
    let info = producer.component_info();

    assert_eq!(info.name, "test_producer");
    assert_eq!(info.type_name, std::any::type_name::<VecProducer<i32>>());
  }

  #[test]
  fn test_component_info_default_name() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let info = producer.component_info();

    assert_eq!(info.name, "vec_producer");
    assert_eq!(info.type_name, std::any::type_name::<VecProducer<i32>>());
  }

  #[tokio::test]
  async fn test_produce_with_default_name() {
    let mut producer = VecProducer::new(vec![1, 2, 3]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_produce_with_custom_name() {
    let mut producer = VecProducer::new(vec![1, 2, 3]).with_name("custom_name".to_string());
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_produce_with_different_types() {
    let mut producer_i32 = VecProducer::new(vec![1, 2, 3]);
    let stream_i32 = producer_i32.produce();
    let result_i32: Vec<i32> = stream_i32.collect().await;
    assert_eq!(result_i32, vec![1, 2, 3]);

    let mut producer_string = VecProducer::new(vec!["a".to_string(), "b".to_string()]);
    let stream_string = producer_string.produce();
    let result_string: Vec<String> = stream_string.collect().await;
    assert_eq!(result_string, vec!["a".to_string(), "b".to_string()]);
  }
}
