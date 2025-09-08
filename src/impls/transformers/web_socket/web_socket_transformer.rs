use crate::error::ErrorStrategy;
use crate::structs::transformers::web_socket::{WebSocketMessage, WebSocketTransformer};
use crate::traits::transformer::TransformerConfig;

impl WebSocketTransformer {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<WebSocketMessage>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }
}
