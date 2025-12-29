use super::transformer::ParentPathTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl Input for ParentPathTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
