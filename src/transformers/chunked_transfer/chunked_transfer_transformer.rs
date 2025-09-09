use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use bytes::Bytes;

pub struct ChunkedTransferTransformer {
  pub config: TransformerConfig<Bytes>,
}

impl Default for ChunkedTransferTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl ChunkedTransferTransformer {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Bytes>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }
}

impl Clone for ChunkedTransferTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}
