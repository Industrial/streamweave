use super::env_var_consumer::EnvVarConsumer;
use streamweave::Input;

impl Input for EnvVarConsumer {
  type Input = (String, String);
  type InputStream = std::pin::Pin<Box<dyn futures::Stream<Item = (String, String)> + Send>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_env_var_consumer_input_stream_type() {
    let _consumer = EnvVarConsumer::new();
    let _input_stream: <EnvVarConsumer as Input>::InputStream =
      Box::pin(stream::iter(vec![("KEY".to_string(), "VALUE".to_string())]));
  }

  #[tokio::test]
  async fn test_env_var_consumer_input_trait_bounds() {
    let _consumer = EnvVarConsumer::new();
    let _: <EnvVarConsumer as Input>::Input = ("KEY".to_string(), "VALUE".to_string());
  }
}
