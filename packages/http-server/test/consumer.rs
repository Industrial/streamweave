#[cfg(test)]
mod tests {
  use axum::http::StatusCode;
  use futures::StreamExt;
  use futures::stream;
  use std::time::Duration;
  use streamweave::error::ErrorStrategy;
  use streamweave_http_server::consumer::{
    HttpResponseConsumer, HttpResponseConsumerConfig, HttpResponseCorrelationConsumer,
  };
  use streamweave_http_server::types::{ContentType, HttpResponse};
  use tokio::sync::mpsc;

  #[test]
  fn test_http_response_consumer_config_default() {
    let config = HttpResponseConsumerConfig::default();
    assert!(!config.stream_response);
    assert_eq!(config.max_items, None);
    assert!(!config.merge_responses);
    assert_eq!(config.default_status, StatusCode::OK);
  }

  #[test]
  fn test_http_response_consumer_config_builder() {
    let config = HttpResponseConsumerConfig::default()
      .with_stream_response(true)
      .with_max_items(Some(10))
      .with_merge_responses(true)
      .with_default_status(StatusCode::CREATED);

    assert!(config.stream_response);
    assert_eq!(config.max_items, Some(10));
    assert!(config.merge_responses);
    assert_eq!(config.default_status, StatusCode::CREATED);
  }

  #[test]
  fn test_http_response_consumer_new() {
    let consumer = HttpResponseConsumer::new();
    assert!(consumer.responses.is_empty());
    assert!(!consumer.finished);
  }

  #[test]
  fn test_http_response_consumer_with_config() {
    let config = HttpResponseConsumerConfig::default().with_stream_response(true);
    let consumer = HttpResponseConsumer::with_config(config);

    assert!(consumer.http_config().stream_response);
  }

  #[test]
  fn test_http_response_consumer_with_error_strategy() {
    let mut consumer = HttpResponseConsumer::new();
    consumer = consumer.with_error_strategy(ErrorStrategy::Skip);
    assert!(matches!(
      consumer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_http_response_consumer_with_name() {
    let consumer = HttpResponseConsumer::new().with_name("test_consumer".to_string());
    assert_eq!(consumer.config.name, "test_consumer");
  }

  #[tokio::test]
  async fn test_http_response_consumer_consume_single() {
    let mut consumer = HttpResponseConsumer::new();

    let response = HttpResponse::text(StatusCode::OK, "Hello");
    let input_stream = stream::iter(vec![response]);
    let boxed_stream = Box::pin(input_stream);

    consumer.consume(boxed_stream).await;

    assert_eq!(consumer.responses.len(), 1);
    assert!(consumer.finished);
  }

  #[tokio::test]
  async fn test_http_response_consumer_consume_multiple() {
    let mut consumer = HttpResponseConsumer::new();

    let responses = vec![
      HttpResponse::text(StatusCode::OK, "First"),
      HttpResponse::text(StatusCode::OK, "Second"),
      HttpResponse::text(StatusCode::OK, "Third"),
    ];
    let input_stream = stream::iter(responses);
    let boxed_stream = Box::pin(input_stream);

    consumer.consume(boxed_stream).await;

    assert_eq!(consumer.responses.len(), 3);
  }

  #[tokio::test]
  async fn test_http_response_consumer_get_response_empty() {
    let mut consumer = HttpResponseConsumer::new();
    let response = consumer.get_response().await;

    assert_eq!(response.status(), StatusCode::OK);
  }

  #[tokio::test]
  async fn test_http_response_consumer_get_response_single() {
    let mut consumer = HttpResponseConsumer::new();

    let http_response = HttpResponse::text(StatusCode::CREATED, "Created");
    let input_stream = stream::iter(vec![http_response]);
    let boxed_stream = Box::pin(input_stream);

    consumer.consume(boxed_stream).await;
    let axum_response = consumer.get_response().await;

    assert_eq!(axum_response.status(), StatusCode::CREATED);
  }

  #[tokio::test]
  async fn test_http_response_consumer_get_response_merge() {
    let config = HttpResponseConsumerConfig::default().with_merge_responses(true);
    let mut consumer = HttpResponseConsumer::with_config(config);

    let responses = vec![
      HttpResponse::text(StatusCode::OK, "First"),
      HttpResponse::text(StatusCode::OK, "Second"),
    ];
    let input_stream = stream::iter(responses);
    let boxed_stream = Box::pin(input_stream);

    consumer.consume(boxed_stream).await;
    let axum_response = consumer.get_response().await;

    assert_eq!(axum_response.status(), StatusCode::OK);
  }

  #[tokio::test]
  async fn test_http_response_consumer_max_items() {
    let config = HttpResponseConsumerConfig::default().with_max_items(Some(2));
    let mut consumer = HttpResponseConsumer::with_config(config);

    let responses = vec![
      HttpResponse::text(StatusCode::OK, "First"),
      HttpResponse::text(StatusCode::OK, "Second"),
      HttpResponse::text(StatusCode::OK, "Third"),
    ];
    let input_stream = stream::iter(responses);
    let boxed_stream = Box::pin(input_stream);

    consumer.consume(boxed_stream).await;

    // Should only collect 2 items due to max_items limit
    assert_eq!(consumer.responses.len(), 2);
  }

  #[tokio::test]
  async fn test_http_response_consumer_merge_responses() {
    let mut consumer = HttpResponseConsumer::new();
    consumer
      .responses
      .push(HttpResponse::text(StatusCode::OK, "First"));
    consumer
      .responses
      .push(HttpResponse::text(StatusCode::OK, "Second"));

    let merged = consumer.merge_responses();
    assert_eq!(merged.body.len(), "FirstSecond".len());
  }

  #[tokio::test]
  async fn test_http_response_consumer_responses() {
    let mut consumer = HttpResponseConsumer::new();
    consumer
      .responses
      .push(HttpResponse::text(StatusCode::OK, "Test"));

    let responses = consumer.responses();
    assert_eq!(responses.len(), 1);
  }

  #[test]
  fn test_http_response_consumer_default() {
    let consumer = HttpResponseConsumer::default();
    assert!(consumer.responses.is_empty());
  }

  #[test]
  fn test_http_response_consumer_clone() {
    let mut consumer = HttpResponseConsumer::new();
    consumer
      .responses
      .push(HttpResponse::text(StatusCode::OK, "Test"));

    let cloned = consumer.clone();
    assert_eq!(cloned.responses.len(), 1);
  }

  #[tokio::test]
  async fn test_http_response_consumer_create_streaming_response() {
    let consumer = HttpResponseConsumer::new();

    let responses = vec![
      HttpResponse::text(StatusCode::OK, "Chunk1"),
      HttpResponse::text(StatusCode::OK, "Chunk2"),
    ];
    let input_stream = stream::iter(responses);

    let axum_response = consumer.create_streaming_response(input_stream).await;

    assert_eq!(axum_response.status(), StatusCode::OK);
    assert!(axum_response.headers().contains_key("transfer-encoding"));
  }

  #[test]
  fn test_http_response_correlation_consumer_new() {
    let consumer = HttpResponseCorrelationConsumer::new();
    // Just verify it creates successfully
    assert!(std::mem::size_of_val(&consumer) > 0);
  }

  #[test]
  fn test_http_response_correlation_consumer_with_timeout() {
    let timeout = Duration::from_secs(60);
    let consumer = HttpResponseCorrelationConsumer::with_timeout(timeout);
    // Just verify it creates successfully
    assert!(std::mem::size_of_val(&consumer) > 0);
  }

  #[test]
  fn test_http_response_correlation_consumer_default() {
    let consumer = HttpResponseCorrelationConsumer::default();
    // Just verify it creates successfully
    assert!(std::mem::size_of_val(&consumer) > 0);
  }

  #[tokio::test]
  async fn test_http_response_correlation_consumer_register_request() {
    let consumer = HttpResponseCorrelationConsumer::new();
    let (tx, _rx) = mpsc::channel(1);

    consumer.register_request("req-123".to_string(), tx).await;
    // Should not panic
  }

  #[tokio::test]
  async fn test_http_response_correlation_consumer_unregister_request() {
    let consumer = HttpResponseCorrelationConsumer::new();
    let (tx, _rx) = mpsc::channel(1);

    consumer.register_request("req-123".to_string(), tx).await;
    consumer.unregister_request("req-123").await;
    // Should not panic
  }

  #[tokio::test]
  async fn test_http_response_correlation_consumer_consume() {
    let consumer = HttpResponseCorrelationConsumer::new();
    let (tx, mut rx) = mpsc::channel(1);

    consumer.register_request("req-123".to_string(), tx).await;

    // Create a message with matching request_id
    let response = HttpResponse::with_request_id(
      StatusCode::OK,
      b"Hello".to_vec(),
      ContentType::Text,
      "req-123".to_string(),
    );
    let message = streamweave::message::Message::new(
      response,
      streamweave::message::MessageId::new_custom("req-123"),
    );

    let input_stream = stream::iter(vec![message]);
    let boxed_stream = Box::pin(input_stream);

    // Consume in a separate task
    let consumer_clone = consumer;
    tokio::spawn(async move {
      let mut consumer = consumer_clone;
      consumer.consume(boxed_stream).await;
    });

    // Wait a bit for the response to be sent
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check if response was received
    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(result.is_ok());
  }
}
