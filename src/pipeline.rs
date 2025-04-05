use crate::traits::{consumer::Consumer, producer::Producer, transformer::Transformer};
use futures::{Stream, StreamExt};
use std::error::Error;
use std::pin::Pin;

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

// Stream type alias to reduce repetition
type BoxedStream<T> = Pin<Box<dyn Stream<Item = Result<T, PipelineError>> + Send>>;

pub struct Pipeline<P, T, C>
where
  P: Producer,
  C: Consumer,
  T: Transformer,
{
  producer: P,
  transformers: Vec<T>,
  consumer: C,
}

impl<P, T, C> Pipeline<P, T, C>
where
  P: Producer,
  C: Consumer,
  T: Transformer,
  // Ensure producer output can be consumed by first transformer
  P::Output: Into<T::Input>,
  // Ensure transformer output can be consumed by consumer
  T::Output: Into<C::Input>,
  // Ensure all errors can be converted to a common error type
  P::Error: Error + Send + 'static,
  T::Error: Error + Send + 'static,
  C::Error: Error + Send + 'static,
  // Add stream type constraints
  T::InputStream: From<P::OutputStream>,
  C::InputStream: From<T::OutputStream>,
{
  pub fn new(producer: P, transformers: Vec<T>, consumer: C) -> Self {
    Self {
      producer,
      transformers,
      consumer,
    }
  }

  pub async fn run(mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Get initial stream from producer
    let mut current_stream = self.producer.produce();

    // Process through transformers
    for transformer in &mut self.transformers {
      let transformed_stream = transformer.transform(T::InputStream::from(current_stream));
      current_stream = transformed_stream;
    }

    // Feed to consumer
    self
      .consumer
      .consume(C::InputStream::from(current_stream))
      .await
      .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::consumers::vec::VecConsumer;
  use crate::producers::vec::VecProducer;
  use crate::transformers::map::MapTransformer;

  #[tokio::test]
  async fn test_pipeline() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let transformer = MapTransformer::new(|x: i32| Ok(x * 2));
    let consumer = VecConsumer::<i32>::new();

    let pipeline = Pipeline::new(producer, vec![transformer], consumer);

    let result = pipeline.run().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_pipeline_error_propagation() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let transformer = MapTransformer::new(|x: i32| {
      if x == 2 {
        Err("Error processing 2".into())
      } else {
        Ok(x * 2)
      }
    });
    let consumer = VecConsumer::<i32>::new();

    let pipeline = Pipeline::new(producer, vec![transformer], consumer);

    let result = pipeline.run().await;
    assert!(result.is_err());
  }
}
