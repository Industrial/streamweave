use super::env_var_producer::EnvVarProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for EnvVarProducer {
  type Output = (String, String);
  type OutputStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
}
