use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use super::http_response_consumer::{
  HttpResponseConsumer, ResponseChunk, StreamingHttpResponseConsumer,
};
use crate::consumer::{Consumer, ConsumerConfig};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl Consumer for HttpResponseConsumer {
  async fn consume(&mut self, mut stream: Self::InputStream) {
    if let Some(response) = stream.next().await {
      let mut sender = self.response_sender.lock().await;
      if let Some(tx) = sender.take() {
        let _ = tx.send(response);
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<Self::Input> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[async_trait]
impl Consumer for StreamingHttpResponseConsumer {
  async fn consume(&mut self, mut stream: Self::InputStream) {
    while let Some(chunk) = stream.next().await {
      match chunk {
        ResponseChunk::Header(status, headers) => {
          // Send response headers
          let _ = self
            .chunk_sender
            .send(ResponseChunk::Header(status, headers))
            .await;
        }
        ResponseChunk::Body(data) => {
          // Send response body chunk
          let _ = self.chunk_sender.send(ResponseChunk::Body(data)).await;
        }
        ResponseChunk::End => {
          // Signal end of response
          let _ = self.chunk_sender.send(ResponseChunk::End).await;
          break;
        }
        ResponseChunk::Error(status, message) => {
          // Send error response
          let _ = self
            .chunk_sender
            .send(ResponseChunk::Error(status, message))
            .await;
          break;
        }
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<Self::Input> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
