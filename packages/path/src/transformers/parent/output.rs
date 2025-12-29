use super::transformer::ParentPathTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for ParentPathTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
