use super::process_consumer::ProcessConsumer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl Input for ProcessConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
