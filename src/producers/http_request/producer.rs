use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::http::{
  connection_info::ConnectionInfo, http_request_chunk::StreamWeaveHttpRequestChunk,
};
use super::http_request_producer::{HttpRequestProducer, StreamingHttpRequestProducer};
use crate::producer::{Producer, ProducerConfig};
use async_trait::async_trait;
use futures::stream;

#[async_trait]
impl Producer for HttpRequestProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let request = self.request.clone();
    Box::pin(stream::once(async move { request }))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Self::Output> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Output>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Output>) -> ErrorContext<Self::Output> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "http_request_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "http_request_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[async_trait]
impl Producer for StreamingHttpRequestProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let method = self.method.clone();
    let uri = self.uri.clone();
    let headers = self.headers.clone();

    // Create a new stream that processes the body stream
    Box::pin(stream::once(async move {
      // Create a default connection info for producer usage
      let connection_info = ConnectionInfo::new(
        "127.0.0.1:8080".parse().unwrap(),
        "0.0.0.0:3000".parse().unwrap(),
        http::Version::HTTP_11,
      );

      // For now, we'll just create a single chunk with the method/uri/headers
      // In a real implementation, this would process the actual body stream
      StreamWeaveHttpRequestChunk::new(
        method,
        uri,
        headers,
        bytes::Bytes::from("Streaming request body"),
        connection_info,
        true, // is_final
      )
    }))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Self::Output> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Output>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Output>) -> ErrorContext<Self::Output> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "streaming_http_request_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "streaming_http_request_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
