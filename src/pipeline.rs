use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineError, StreamError,
};
use crate::traits::{consumer::Consumer, producer::Producer, transformer::Transformer};
use chrono::Utc;
use std::marker::PhantomData;

// State types for the builder
pub struct Empty;
pub struct HasProducer<P>(PhantomData<P>);
pub struct HasTransformer<P, T>(PhantomData<(P, T)>);
pub struct Complete<P, T, C>(PhantomData<(P, T, C)>);

// Pipeline builder with state and error handling
pub struct PipelineBuilder<State> {
  producer_stream: Option<Box<dyn std::any::Any + Send + 'static>>,
  transformer_stream: Option<Box<dyn std::any::Any + Send + 'static>>,
  consumer: Option<Box<dyn std::any::Any + Send + 'static>>,
  error_strategy: ErrorStrategy<()>,
  _state: State,
}

// Pipeline struct that holds the final state
pub struct Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  producer_stream: Option<P::OutputStream>,
  transformer_stream: Option<T::OutputStream>,
  consumer: Option<C>,
  error_strategy: ErrorStrategy<()>,
}

// Initial builder creation
impl PipelineBuilder<Empty> {
  pub fn new() -> Self {
    PipelineBuilder {
      producer_stream: None,
      transformer_stream: None,
      consumer: None,
      error_strategy: ErrorStrategy::Stop,
      _state: Empty,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<()>) -> Self {
    self.error_strategy = strategy;
    self
  }

  pub fn producer<P>(mut self, mut producer: P) -> PipelineBuilder<HasProducer<P>>
  where
    P: Producer + 'static,
    P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    P::OutputStream: 'static,
  {
    let stream = producer.produce();
    self.producer_stream = Some(Box::new(stream));

    PipelineBuilder {
      producer_stream: self.producer_stream,
      transformer_stream: None,
      consumer: None,
      error_strategy: self.error_strategy,
      _state: HasProducer(PhantomData),
    }
  }
}

impl Default for PipelineBuilder<Empty> {
  fn default() -> Self {
    Self::new()
  }
}

// After producer is added
impl<P> PipelineBuilder<HasProducer<P>>
where
  P: Producer + 'static,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  P::OutputStream: 'static,
{
  pub fn transformer<T>(mut self, mut transformer: T) -> PipelineBuilder<HasTransformer<P, T>>
  where
    T: Transformer + 'static,
    T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::InputStream: From<P::OutputStream>,
    T::OutputStream: 'static,
  {
    let producer_stream = self
      .producer_stream
      .take()
      .unwrap()
      .downcast::<P::OutputStream>()
      .unwrap();

    let transformer_stream = transformer.transform((*producer_stream).into());
    self.transformer_stream = Some(Box::new(transformer_stream));

    PipelineBuilder {
      producer_stream: None,
      transformer_stream: self.transformer_stream,
      consumer: None,
      error_strategy: self.error_strategy,
      _state: HasTransformer(PhantomData),
    }
  }
}

// After transformer is added
impl<P, T> PipelineBuilder<HasTransformer<P, T>>
where
  P: Producer + 'static,
  T: Transformer + 'static,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::OutputStream: 'static,
{
  pub fn transformer<U>(mut self, mut transformer: U) -> PipelineBuilder<HasTransformer<P, U>>
  where
    U: Transformer + 'static,
    U::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    U::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    U::InputStream: From<T::OutputStream>,
    U::OutputStream: 'static,
  {
    let transformer_stream = self
      .transformer_stream
      .take()
      .unwrap()
      .downcast::<T::OutputStream>()
      .unwrap();

    let new_stream = transformer.transform((*transformer_stream).into());
    self.transformer_stream = Some(Box::new(new_stream));

    PipelineBuilder {
      producer_stream: None,
      transformer_stream: self.transformer_stream,
      consumer: None,
      error_strategy: self.error_strategy,
      _state: HasTransformer(PhantomData),
    }
  }

  pub fn consumer<C>(mut self, consumer: C) -> Pipeline<P, T, C>
  where
    C: Consumer + 'static,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    C::InputStream: From<T::OutputStream>,
  {
    let transformer_stream = self
      .transformer_stream
      .take()
      .unwrap()
      .downcast::<T::OutputStream>()
      .unwrap();

    Pipeline {
      producer_stream: None,
      transformer_stream: Some(*transformer_stream),
      consumer: Some(consumer),
      error_strategy: self.error_strategy,
    }
  }
}

impl<P, T, C> Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<()>) -> Self {
    self.error_strategy = strategy;
    self
  }

  async fn handle_error(&self, error: StreamError<()>) -> Result<ErrorAction, PipelineError<()>> {
    match &self.error_strategy {
      ErrorStrategy::Stop => Ok(ErrorAction::Stop),
      ErrorStrategy::Skip => Ok(ErrorAction::Skip),
      ErrorStrategy::Retry(max_retries) => {
        if error.retries < *max_retries {
          Ok(ErrorAction::Retry)
        } else {
          Ok(ErrorAction::Stop)
        }
      }
      ErrorStrategy::Custom(handler) => Ok(handler(&error)),
    }
  }

  pub async fn run(mut self) -> Result<((), C), PipelineError<()>>
  where
    C::InputStream: From<T::OutputStream>,
  {
    let transformer_stream = self.transformer_stream.take().unwrap();
    let mut consumer = self.consumer.take().unwrap();

    consumer.consume(transformer_stream.into()).await;
    Ok(((), consumer))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::{
    consumer::{Consumer, ConsumerConfig},
    input::Input,
    output::Output,
    producer::{Producer, ProducerConfig},
    transformer::{Transformer, TransformerConfig},
  };
  use async_trait::async_trait;
  use futures::{Stream, StreamExt};
  use std::pin::Pin;

  // Test error types
  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  // Mock Producer
  #[derive(Clone)]
  struct NumberProducer {
    numbers: Vec<i32>,
    config: ProducerConfig<i32>,
  }

  impl NumberProducer {
    fn new(numbers: Vec<i32>) -> Self {
      Self {
        numbers,
        config: ProducerConfig::default(),
      }
    }
  }

  impl Output for NumberProducer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
  }

  impl Producer for NumberProducer {
    fn produce(&mut self) -> Self::OutputStream {
      Box::pin(futures::stream::iter(self.numbers.clone()))
    }

    fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> &ProducerConfig<Self::Output> {
      &self.config
    }

    fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output> {
      &mut self.config
    }
  }

  // Mock Transformer
  #[derive(Clone)]
  struct DoubleTransformer {
    config: TransformerConfig<i32>,
  }

  impl DoubleTransformer {
    fn new() -> Self {
      Self {
        config: TransformerConfig::default(),
      }
    }
  }

  impl Input for DoubleTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
  }

  impl Output for DoubleTransformer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
  }

  impl Transformer for DoubleTransformer {
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      Box::pin(input.map(|x| x * 2))
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

  // Mock Consumer
  #[derive(Clone)]
  struct CollectConsumer {
    items: Vec<i32>,
    config: ConsumerConfig<i32>,
  }

  impl CollectConsumer {
    fn new() -> Self {
      Self {
        items: Vec::new(),
        config: ConsumerConfig::default(),
      }
    }
  }

  impl Input for CollectConsumer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
  }

  #[async_trait]
  impl Consumer for CollectConsumer {
    async fn consume(&mut self, input: Self::InputStream) {
      let mut items = Vec::new();
      input.for_each(|item| items.push(item)).await;
      self.items = items;
    }

    fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> ConsumerConfig<Self::Input> {
      self.config.clone()
    }
  }

  #[tokio::test]
  async fn test_pipeline() {
    let producer = NumberProducer::new(vec![1, 2, 3]);
    let transformer = DoubleTransformer::new();
    let consumer = CollectConsumer::new();

    let pipeline = PipelineBuilder::new()
      .producer(producer)
      .transformer(transformer)
      .consumer(consumer);

    let ((), mut consumer) = pipeline.run().await.unwrap();
    assert_eq!(consumer.items, vec![2, 4, 6]);
  }
}
