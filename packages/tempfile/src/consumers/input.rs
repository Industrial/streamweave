use super::tempfile_consumer::TempFileConsumer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl Input for TempFileConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
