use super::transformer::FileNameTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for FileNameTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
