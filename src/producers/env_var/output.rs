use super::env_var_producer::EnvVarProducer;
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for EnvVarProducer {
  type Output = (String, String);
  type OutputStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
}
