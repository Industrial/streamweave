use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::http::http_response::StreamWeaveHttpResponse;

/// HTTP response consumer for handling single responses
#[derive(Debug, Clone)]
pub struct HttpResponseConsumer {
  pub response_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<StreamWeaveHttpResponse>>>>,
  pub config: ConsumerConfig<StreamWeaveHttpResponse>,
}

impl HttpResponseConsumer {
  pub fn new() -> (
    Self,
    tokio::sync::oneshot::Receiver<StreamWeaveHttpResponse>,
  ) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let consumer = Self {
      response_sender: Arc::new(Mutex::new(Some(tx))),
      config: ConsumerConfig::default(),
    };
    (consumer, rx)
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<StreamWeaveHttpResponse>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bytes::Bytes;

  #[tokio::test]
  async fn test_http_response_consumer_new() {
    let (consumer, receiver) = HttpResponseConsumer::new();

    // Send a test response
    let response = StreamWeaveHttpResponse::ok(Bytes::from("test"));
    let sender = consumer.response_sender.lock().await.take().unwrap();
    let _ = sender.send(response.clone());

    // Receive the response
    let received = receiver.await.unwrap();
    assert_eq!(received.status, response.status);
    assert_eq!(received.body, response.body);
  }

  #[test]
  fn test_with_error_strategy() {
    use crate::error::ErrorAction;
    let strategy = ErrorStrategy::<StreamWeaveHttpResponse>::new_custom(|_| ErrorAction::Skip);
    let (consumer, _) = HttpResponseConsumer::new();
    let consumer = consumer.with_error_strategy(strategy.clone());

    assert_eq!(consumer.config.error_strategy, strategy);
  }

  #[test]
  fn test_with_name() {
    let name = "test_consumer".to_string();
    let (consumer, _) = HttpResponseConsumer::new();
    let consumer = consumer.with_name(name.clone());

    assert_eq!(consumer.config.name, name);
  }

  #[tokio::test]
  async fn test_response_processing() {
    // Test different response types
    let responses = vec![
      StreamWeaveHttpResponse::ok(Bytes::from("success")),
      StreamWeaveHttpResponse::not_found(Bytes::from("not found")),
      StreamWeaveHttpResponse::internal_server_error(Bytes::from("error")),
    ];

    for expected_response in responses {
      let (consumer, receiver) = HttpResponseConsumer::new();
      let sender = consumer.response_sender.lock().await.take().unwrap();
      let _ = sender.send(expected_response.clone());

      let received = receiver.await.unwrap();
      assert_eq!(received.status, expected_response.status);
      assert_eq!(received.body, expected_response.body);
    }
  }
}
