use crate::traits::{consumer::Consumer, producer::Producer, transformer::Transformer};
use std::error::Error;
use std::marker::PhantomData;

// Common error type for the pipeline
#[derive(Debug)]
pub struct PipelineError {
  inner: Box<dyn Error + Send + Sync>,
}

impl PipelineError {
  pub fn new<E>(error: E) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    Self {
      inner: Box::new(error),
    }
  }
}

impl std::fmt::Display for PipelineError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.inner)
  }
}

impl Error for PipelineError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(&*self.inner)
  }
}

// State types for the builder
pub struct Empty;
pub struct HasProducer<P>(PhantomData<P>);
pub struct HasTransformer<P, T>(PhantomData<(P, T)>);
pub struct Complete<P, T, C>(PhantomData<(P, T, C)>);

// Pipeline builder with state
pub struct PipelineBuilder<State> {
  producer_stream: Option<Box<dyn std::any::Any + Send + 'static>>,
  transformer_stream: Option<Box<dyn std::any::Any + Send + 'static>>,
  consumer: Option<Box<dyn std::any::Any + Send + 'static>>,
  _state: State,
}

// Pipeline struct that holds the final state
pub struct Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
{
  producer_stream: Option<P::OutputStream>,
  transformer_stream: Option<T::OutputStream>,
  consumer: Option<C>,
}

// Create a standalone builder function
pub fn build() -> PipelineBuilder<Empty> {
  PipelineBuilder::new()
}

// Initial builder creation
impl PipelineBuilder<Empty> {
  fn new() -> Self {
    PipelineBuilder {
      producer_stream: None,
      transformer_stream: None,
      consumer: None,
      _state: Empty,
    }
  }

  pub fn producer<P>(mut self, mut producer: P) -> PipelineBuilder<HasProducer<P>>
  where
    P: Producer + 'static,
    P::OutputStream: 'static,
  {
    let stream = producer.produce();
    self.producer_stream = Some(Box::new(stream));

    PipelineBuilder {
      producer_stream: self.producer_stream,
      transformer_stream: None,
      consumer: None,
      _state: HasProducer(PhantomData),
    }
  }
}

// After producer is added
impl<P> PipelineBuilder<HasProducer<P>>
where
  P: Producer + 'static,
  P::OutputStream: 'static,
{
  pub fn transformer<T>(mut self, mut transformer: T) -> PipelineBuilder<HasTransformer<P, T>>
  where
    T: Transformer<Input = P::Output> + 'static,
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
      _state: HasTransformer(PhantomData),
    }
  }
}

// After transformer is added
impl<P, T> PipelineBuilder<HasTransformer<P, T>>
where
  P: Producer + 'static,
  T: Transformer + 'static,
  T::OutputStream: 'static,
{
  pub fn transformer<U>(mut self, mut transformer: U) -> PipelineBuilder<HasTransformer<P, U>>
  where
    U: Transformer<Input = T::Output> + 'static,
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
      _state: HasTransformer(PhantomData),
    }
  }

  pub fn consumer<C>(mut self, consumer: C) -> Pipeline<P, T, C>
  where
    C: Consumer<Input = T::Output>,
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
    }
  }
}

impl<P, T, C> Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
{
  pub async fn run(mut self) -> Result<((), C), PipelineError>
  where
    C::InputStream: From<T::OutputStream>,
  {
    let mut consumer = self.consumer.take().unwrap();
    let transformer_stream = self.transformer_stream.take().unwrap();

    consumer
      .consume(transformer_stream.into())
      .await
      .map_err(PipelineError::new)
      .map(|()| ((), consumer))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::{error::Error as StreamError, input::Input, output::Output};
  use async_trait::async_trait;
  use futures::{Stream, StreamExt};
  use std::pin::Pin;

  // Test error types
  #[derive(Debug)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  impl StreamError for TestError {
    type Error = Self;
  }

  // Mock Producer
  struct NumberProducer {
    numbers: Vec<i32>,
  }

  impl StreamError for NumberProducer {
    type Error = TestError;
  }

  impl Output for NumberProducer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
  }

  impl Producer for NumberProducer {
    fn produce(&mut self) -> Self::OutputStream {
      let numbers = self.numbers.clone();
      Box::pin(futures::stream::iter(numbers).map(Ok))
    }
  }

  // Mock Transformers
  struct DoubleTransformer;

  impl StreamError for DoubleTransformer {
    type Error = TestError;
  }

  impl Input for DoubleTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
  }

  impl Output for DoubleTransformer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
  }

  impl Transformer for DoubleTransformer {
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      Box::pin(input.map(|r| r.map(|n| n * 2)))
    }
  }

  struct StringifyTransformer;

  impl StreamError for StringifyTransformer {
    type Error = TestError;
  }

  impl Input for StringifyTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
  }

  impl Output for StringifyTransformer {
    type Output = String;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
  }

  impl Transformer for StringifyTransformer {
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      Box::pin(input.map(|r| r.map(|n| n.to_string())))
    }
  }

  // Mock Consumer
  struct CollectConsumer {
    collected: Vec<String>,
  }

  impl StreamError for CollectConsumer {
    type Error = TestError;
  }

  impl Input for CollectConsumer {
    type Input = String;
    type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
  }

  #[async_trait]
  impl Consumer for CollectConsumer {
    async fn consume(&mut self, mut input: Self::InputStream) -> Result<(), Self::Error> {
      while let Some(result) = input.next().await {
        self.collected.push(result?);
      }
      Ok(())
    }
  }

  #[tokio::test]
  async fn test_basic_pipeline() {
    let producer = NumberProducer {
      numbers: vec![1, 2, 3],
    };
    let transformer = StringifyTransformer;
    let consumer = CollectConsumer {
      collected: Vec::new(),
    };

    let (_, consumer) = build()
      .producer(producer)
      .transformer(transformer)
      .consumer(consumer)
      .run()
      .await
      .unwrap();

    assert_eq!(consumer.collected, vec!["1", "2", "3"]);
  }

  #[tokio::test]
  async fn test_chained_transformers() {
    let producer = NumberProducer {
      numbers: vec![1, 2, 3],
    };
    let double_transformer = DoubleTransformer;
    let stringify_transformer = StringifyTransformer;
    let consumer = CollectConsumer {
      collected: Vec::new(),
    };

    let (_, consumer) = build()
      .producer(producer)
      .transformer(double_transformer)
      .transformer(stringify_transformer)
      .consumer(consumer)
      .run()
      .await
      .unwrap();

    assert_eq!(consumer.collected, vec!["2", "4", "6"]);
  }

  #[tokio::test]
  async fn test_empty_stream() {
    let producer = NumberProducer { numbers: vec![] };
    let transformer = StringifyTransformer;
    let consumer = CollectConsumer {
      collected: Vec::new(),
    };

    let (_, consumer) = build()
      .producer(producer)
      .transformer(transformer)
      .consumer(consumer)
      .run()
      .await
      .unwrap();

    assert!(consumer.collected.is_empty());
  }

  #[tokio::test]
  async fn test_error_propagation() {
    struct ErrorProducer;

    impl StreamError for ErrorProducer {
      type Error = TestError;
    }

    impl Output for ErrorProducer {
      type Output = i32;
      type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
    }

    impl Producer for ErrorProducer {
      fn produce(&mut self) -> Self::OutputStream {
        Box::pin(futures::stream::once(async {
          Err(TestError("Producer error".to_string()))
        }))
      }
    }

    let producer = ErrorProducer;
    let transformer = StringifyTransformer;
    let consumer = CollectConsumer {
      collected: Vec::new(),
    };

    let result = build()
      .producer(producer)
      .transformer(transformer)
      .consumer(consumer)
      .run()
      .await;

    assert!(result.is_err());
  }
}
