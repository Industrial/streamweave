use futures::{Stream, stream};
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::ErrorStrategy;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};

/// A producer that yields items from a Vec.
///
/// This producer iterates over all items in the Vec and produces them
/// in order.
#[derive(Clone)]
pub struct VecProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The Vec data to produce from.
  pub data: Vec<T>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> VecProducer<T> {
  /// Creates a new `VecProducer` with the given Vec.
  ///
  /// # Arguments
  ///
  /// * `data` - The Vec to produce items from.
  pub fn new(data: Vec<T>) -> Self {
    Self {
      data,
      config: streamweave::ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for VecProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

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
