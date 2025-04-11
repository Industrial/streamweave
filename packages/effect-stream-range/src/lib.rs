use effect_stream::EffectStream;
use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct RangeError(String);

impl fmt::Display for RangeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Range error: {}", self.0)
  }
}

impl Error for RangeError {}

pub struct RangeStream {
  stream: EffectStream<i32, RangeError>,
}

impl RangeStream {
  pub fn new(range: RangeInclusive<i32>) -> Self {
    let stream = EffectStream::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in range {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    Self { stream }
  }

  pub async fn next(&self) -> Result<Option<i32>, effect_stream::EffectError<RangeError>> {
    self.stream.next().await
  }
}
