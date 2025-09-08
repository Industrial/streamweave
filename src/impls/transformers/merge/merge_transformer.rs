use crate::error::ErrorStrategy;
use crate::structs::transformers::merge::MergeTransformer;
use crate::traits::transformer::TransformerConfig;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

impl<T> Clone for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      _phantom: self._phantom,
      config: self.config.clone(),
      streams: Vec::new(), // Streams can't be cloned, so start with empty
    }
  }
}

impl<T> MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: PhantomData,
      config: TransformerConfig::default(),
      streams: Vec::new(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  pub fn add_stream(&mut self, stream: Pin<Box<dyn Stream<Item = T> + Send>>) {
    self.streams.push(stream);
  }
}

impl<T> Default for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}
