use crate::error::ErrorStrategy;
use crate::structs::transformers::server_sent_events::ServerSentEventsTransformer;
use crate::traits::transformer::TransformerConfig;

impl Default for ServerSentEventsTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl ServerSentEventsTransformer {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  pub fn with_error_strategy(
    mut self,
    strategy: ErrorStrategy<crate::structs::transformers::server_sent_events::SSEMessage>,
  ) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }
}

impl Clone for ServerSentEventsTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}
