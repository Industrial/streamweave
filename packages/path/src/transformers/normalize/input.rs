use super::transformer::NormalizePathTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl Input for NormalizePathTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
