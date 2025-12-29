use super::transformer::FileNameTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl Input for FileNameTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
