use effect_stream::EffectStream;
use serde_json::Value;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct JsonError(String);

impl fmt::Display for JsonError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "JSON error: {}", self.0)
  }
}

impl Error for JsonError {}

pub struct JsonStream {
  stream: EffectStream<Value, JsonError>,
}

impl JsonStream {
  pub fn new(input: impl Into<String>) -> Self {
    let stream = EffectStream::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      let input = input.into();
      match serde_json::from_str::<Value>(&input) {
        Ok(value) => {
          stream_clone.push(value).await.unwrap();
          stream_clone.close().await.unwrap();
        }
        Err(e) => {
          stream_clone
            .push_error(JsonError(e.to_string()))
            .await
            .unwrap();
        }
      }
    });

    Self { stream }
  }

  pub async fn next(&self) -> Result<Option<Value>, JsonError> {
    self.stream.next().await
  }
}
