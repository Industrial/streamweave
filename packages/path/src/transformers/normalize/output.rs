use super::transformer::NormalizePathTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for NormalizePathTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
