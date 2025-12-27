use super::vec_producer::VecProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for VecProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
