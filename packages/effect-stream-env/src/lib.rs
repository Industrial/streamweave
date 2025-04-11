use effect_stream::EffectStream;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct EnvError(String);

impl fmt::Display for EnvError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Environment error: {}", self.0)
  }
}

impl Error for EnvError {}

pub struct EnvStream {
  stream: EffectStream<String, EnvError>,
}

impl EnvStream {
  pub fn new(var_name: impl Into<String>) -> Self {
    let stream = EffectStream::new();
    let mut stream_clone = stream.clone();
    let var_name = var_name.into();

    tokio::spawn(async move {
      match std::env::var(&var_name) {
        Ok(value) => {
          stream_clone.push(value).await.unwrap();
          stream_clone.close().await.unwrap();
        }
        Err(e) => {
          stream_clone
            .push_error(EnvError(e.to_string()))
            .await
            .unwrap();
        }
      }
    });

    Self { stream }
  }

  pub async fn next(&self) -> Result<Option<String>, effect_stream::EffectError<EnvError>> {
    self.stream.next().await
  }
}
