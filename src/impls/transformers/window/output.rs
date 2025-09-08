use crate::structs::transformers::window::WindowTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for WindowTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}
