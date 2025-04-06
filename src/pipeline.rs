use crate::traits::{consumer::Consumer, producer::Producer, transformer::Transformer};
use std::error::Error;

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

impl<P, T, C> Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
{
  pub fn new() -> Self {
    Pipeline {
      producer_stream: None,
      transformer_stream: None,
      consumer: None,
    }
  }

  pub fn producer(self, mut producer: P) -> Self {
    let stream = producer.produce();
    Pipeline {
      producer_stream: Some(stream),
      transformer_stream: None,
      consumer: None,
    }
  }

  pub fn transformer<U>(self, mut transformer: U) -> Pipeline<P, U, C>
  where
    U: Transformer<Input = P::Output>,
    U::InputStream: From<P::OutputStream>,
  {
    if self.producer_stream.is_none() {
      panic!("Producer stream is not set");
    }

    let producer_stream = self.producer_stream.unwrap();
    let transformer_stream = transformer.transform(producer_stream.into());
    Pipeline {
      producer_stream: None,
      transformer_stream: Some(transformer_stream),
      consumer: None,
    }
  }

  pub fn consumer<U>(self, consumer: U) -> Pipeline<P, T, U>
  where
    U: Consumer<Input = T::Output>,
    U::InputStream: From<T::OutputStream>,
  {
    if self.producer_stream.is_none() {
      panic!("Producer stream is not set");
    }

    if self.transformer_stream.is_none() {
      panic!("Transformer stream is not set");
    }

    Pipeline {
      producer_stream: self.producer_stream,
      transformer_stream: self.transformer_stream,
      consumer: Some(consumer),
    }
  }

  pub async fn run(self) -> Result<(), PipelineError>
  where
    C::InputStream: From<T::OutputStream>,
  {
    if self.consumer.is_none() {
      panic!("No consumer set");
    }

    if self.transformer_stream.is_none() {
      panic!("No transformer stream set");
    }

    let mut consumer = self.consumer.unwrap();

    consumer
      .consume(self.transformer_stream.unwrap().into())
      .await
      .map_err(PipelineError::new)
  }
}
