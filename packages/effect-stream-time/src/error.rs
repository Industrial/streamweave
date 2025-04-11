use effect_stream::EffectError;
use std::fmt;

/// Errors that can occur in the time stream package
#[derive(Debug, Clone)]
pub enum TimeStreamError {
  /// An error occurred in the underlying stream
  StreamError(EffectError<Self>),
  /// The stream interval is zero
  ZeroInterval,
}

impl fmt::Display for TimeStreamError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::StreamError(e) => write!(f, "Stream error: {}", e),
      Self::ZeroInterval => write!(f, "Time stream interval cannot be zero"),
    }
  }
}

impl std::error::Error for TimeStreamError {}

impl From<EffectError<Self>> for TimeStreamError {
  fn from(e: EffectError<Self>) -> Self {
    Self::StreamError(e)
  }
}
