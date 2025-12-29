use super::directory_consumer::DirectoryConsumer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl Input for DirectoryConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
