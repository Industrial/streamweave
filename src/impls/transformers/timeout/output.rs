use crate::structs::transformers::timeout::TimeoutTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TimeoutTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
