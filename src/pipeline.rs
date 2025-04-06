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
  producer: Option<P>,
  producer_stream: Option<P::OutputStream>,
  transformer: Option<T>,
  transformer_stream: Option<T::OutputStream>,
  consumer: Option<C>,
  consumer_stream: Option<C::InputStream>,
}

impl<P, T, C> Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
{
  pub fn new() -> Self {
    Pipeline {
      producer: None,
      producer_stream: None,
      transformer: None,
      transformer_stream: None,
      consumer: None,
      consumer_stream: None,
    }
  }

  pub fn producer(self, producer: P) -> Self {
    Pipeline {
      producer: Some(producer),
      producer_stream: Some(producer.produce()),
      transformer: self.transformer,
      transformer_stream: None,
      consumer: self.consumer,
      consumer_stream: None,
    }
  }

  pub fn transformer(self, transformer: T) -> Self {
    if self.producer_stream.is_none() {
      panic!("Producer stream is not set");
    }

    if self.transformer_stream.is_none() {
      Pipeline {
        producer: self.producer,
        producer_stream: self.producer_stream,
        transformer: Some(transformer),
        transformer_stream: Some(transformer.transform(self.producer_stream.unwrap())),
        consumer: self.consumer,
        consumer_stream: None,
      }
    } else {
      Pipeline {
        producer: self.producer,
        producer_stream: self.producer_stream,
        transformer: Some(transformer),
        transformer_stream: Some(transformer.transform(self.transformer_stream.unwrap())),
        consumer: self.consumer,
        consumer_stream: None,
      }
    }
  }

  pub fn consumer(self, consumer: C) -> Self {
    if self.producer_stream.is_none() {
      panic!("Producer stream is not set");
    }

    if self.transformer_stream.is_none() {
      panic!("Transformer stream is not set");
    }

    Pipeline {
      producer: self.producer,
      producer_stream: self.producer_stream,
      transformer: self.transformer,
      transformer_stream: self.transformer_stream,
      consumer: Some(consumer),
      consumer_stream: Some(consumer.consume(self.transformer_stream.unwrap())),
    }
  }

  pub async fn run(self) -> Result<(), PipelineError> {
    self
      .consumer_stream
      .await
      .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
  }
}
