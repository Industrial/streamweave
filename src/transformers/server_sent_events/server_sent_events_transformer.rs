use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;

#[derive(Debug, Clone)]
pub struct SSEMessage {
  pub event: Option<String>,
  pub data: String,
  pub id: Option<String>,
  pub retry: Option<u64>,
}

impl SSEMessage {
  pub fn new(data: String) -> Self {
    Self {
      event: None,
      data,
      id: None,
      retry: None,
    }
  }

  pub fn with_event(mut self, event: String) -> Self {
    self.event = Some(event);
    self
  }

  pub fn with_id(mut self, id: String) -> Self {
    self.id = Some(id);
    self
  }

  pub fn with_retry(mut self, retry: u64) -> Self {
    self.retry = Some(retry);
    self
  }

  pub fn to_sse_format(&self) -> String {
    let mut sse_text = String::new();

    if let Some(event) = &self.event {
      sse_text.push_str(&format!("event: {}\n", event));
    }
    if let Some(id) = &self.id {
      sse_text.push_str(&format!("id: {}\n", id));
    }
    if let Some(retry) = self.retry {
      sse_text.push_str(&format!("retry: {}\n", retry));
    }

    sse_text.push_str(&format!("data: {}\n\n", self.data));
    sse_text
  }
}

pub struct ServerSentEventsTransformer {
  pub config: TransformerConfig<SSEMessage>,
}

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
    strategy: ErrorStrategy<crate::transformers::server_sent_events::server_sent_events_transformer::SSEMessage>,
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
