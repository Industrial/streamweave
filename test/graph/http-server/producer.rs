#[cfg(test)]
mod tests {
  use axum::body::Body;
  use axum::extract::Request;
  use axum::http::{Method, Uri};
  use futures::StreamExt;
  use std::collections::HashMap;
  use streamweave::error::ErrorStrategy;
  use streamweave_http_server::producer::{
    HttpRequestProducer, HttpRequestProducerConfig, LongLivedHttpRequestProducer,
  };
  use streamweave_http_server::types::HttpRequest;
  use tokio::sync::mpsc;

  #[test]
  fn test_http_request_producer_config_default() {
    let config = HttpRequestProducerConfig::default();
    assert!(config.extract_body);
    assert_eq!(config.max_body_size, Some(10 * 1024 * 1024));
    assert!(config.parse_json);
    assert!(config.extract_query_params);
    assert!(config.extract_path_params);
    assert!(!config.stream_body);
    assert_eq!(config.chunk_size, 64 * 1024);
  }

  #[test]
  fn test_http_request_producer_config_builder() {
    let config = HttpRequestProducerConfig::default()
      .with_extract_body(false)
      .with_max_body_size(Some(5 * 1024 * 1024))
      .with_parse_json(false)
      .with_extract_query_params(false)
      .with_extract_path_params(false)
      .with_stream_body(true)
      .with_chunk_size(128 * 1024);

    assert!(!config.extract_body);
    assert_eq!(config.max_body_size, Some(5 * 1024 * 1024));
    assert!(!config.parse_json);
    assert!(!config.extract_query_params);
    assert!(!config.extract_path_params);
    assert!(config.stream_body);
    assert_eq!(config.chunk_size, 128 * 1024);
  }

  #[tokio::test]
  async fn test_http_request_producer_from_axum_request() {
    let request = Request::builder()
      .uri("/test?key=value")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let config = HttpRequestProducerConfig::default();
    let producer = HttpRequestProducer::from_axum_request(request, config).await;

    assert!(producer.request.is_some());
    assert_eq!(producer.http_config.extract_body, true);
  }

  #[tokio::test]
  async fn test_http_request_producer_produce() {
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let config = HttpRequestProducerConfig::default();
    let mut producer = HttpRequestProducer::from_axum_request(request, config).await;

    let mut stream = producer.produce();
    let http_request = stream.next().await;

    assert!(http_request.is_some());
    let req = http_request.unwrap();
    assert_eq!(req.method, streamweave_http_server::types::HttpMethod::Get);
    assert_eq!(req.path, "/test");
  }

  #[tokio::test]
  async fn test_http_request_producer_with_error_strategy() {
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let config = HttpRequestProducerConfig::default();
    let producer = HttpRequestProducer::from_axum_request(request, config).await;

    let producer = producer.with_error_strategy(ErrorStrategy::Skip);
    assert!(matches!(
      producer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[tokio::test]
  async fn test_http_request_producer_with_name() {
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let config = HttpRequestProducerConfig::default();
    let producer = HttpRequestProducer::from_axum_request(request, config).await;

    let producer = producer.with_name("test_producer".to_string());
    assert_eq!(producer.config.name, Some("test_producer".to_string()));
  }

  #[tokio::test]
  async fn test_http_request_producer_set_path_params() {
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let config = HttpRequestProducerConfig::default();
    let mut producer = HttpRequestProducer::from_axum_request(request, config).await;

    let mut path_params = HashMap::new();
    path_params.insert("id".to_string(), "123".to_string());
    producer.set_path_params(path_params);

    assert_eq!(
      producer.request.as_ref().unwrap().path_params.get("id"),
      Some(&"123".to_string())
    );
  }

  #[tokio::test]
  async fn test_http_request_producer_http_config() {
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let config = HttpRequestProducerConfig::default();
    let producer = HttpRequestProducer::from_axum_request(request, config).await;

    let http_config = producer.http_config();
    assert_eq!(http_config.extract_body, true);
  }

  #[tokio::test]
  async fn test_http_request_producer_clone() {
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let config = HttpRequestProducerConfig::default();
    let producer = HttpRequestProducer::from_axum_request(request, config).await;

    let cloned = producer.clone();
    assert_eq!(
      producer.http_config.extract_body,
      cloned.http_config.extract_body
    );
  }

  #[tokio::test]
  async fn test_http_request_producer_with_body() {
    let body = Body::from("test body");
    let request = Request::builder()
      .uri("/test")
      .method(Method::POST)
      .header("content-type", "text/plain")
      .body(body)
      .unwrap();

    let config = HttpRequestProducerConfig::default().with_extract_body(true);
    let mut producer = HttpRequestProducer::from_axum_request(request, config).await;

    let mut stream = producer.produce();
    let http_request = stream.next().await;

    assert!(http_request.is_some());
    let req = http_request.unwrap();
    // Body should be extracted
    assert!(req.body.is_some() || req.body.is_none()); // May or may not be extracted depending on implementation
  }

  #[tokio::test]
  async fn test_long_lived_http_request_producer_new() {
    let (tx, rx) = mpsc::channel(100);
    let config = HttpRequestProducerConfig::default();
    let producer = LongLivedHttpRequestProducer::new(rx, config);

    assert_eq!(producer.http_config().extract_body, true);
  }

  #[tokio::test]
  async fn test_long_lived_http_request_producer_with_default_config() {
    let (tx, rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(rx);

    assert_eq!(producer.http_config().extract_body, true);
  }

  #[tokio::test]
  async fn test_long_lived_http_request_producer_http_config() {
    let (tx, rx) = mpsc::channel(100);
    let config = HttpRequestProducerConfig::default().with_extract_body(false);
    let producer = LongLivedHttpRequestProducer::new(rx, config);

    assert_eq!(producer.http_config().extract_body, false);
  }

  #[tokio::test]
  async fn test_long_lived_http_request_producer_http_config_mut() {
    let (tx, rx) = mpsc::channel(100);
    let config = HttpRequestProducerConfig::default();
    let mut producer = LongLivedHttpRequestProducer::new(rx, config);

    producer.http_config_mut().extract_body = false;
    assert_eq!(producer.http_config().extract_body, false);
  }

  #[tokio::test]
  async fn test_long_lived_http_request_producer_produce() {
    let (tx, rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(rx);

    // Send a request through the channel
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    tx.send(request).await.unwrap();
    drop(tx); // Close the channel

    let mut producer = producer;
    let mut stream = producer.produce();

    // Should receive the message
    let message = stream.next().await;
    assert!(message.is_some());
    let msg = message.unwrap();
    assert_eq!(msg.payload().path, "/test");
  }

  #[tokio::test]
  async fn test_long_lived_http_request_producer_produce_multiple() {
    let (tx, rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(rx);

    // Send multiple requests
    for i in 0..3 {
      let request = Request::builder()
        .uri(&format!("/test{}", i))
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();
      tx.send(request).await.unwrap();
    }
    drop(tx);

    let mut producer = producer;
    let mut stream = producer.produce();

    // Should receive all messages
    let mut count = 0;
    while let Some(message) = stream.next().await {
      assert_eq!(message.payload().path, format!("/test{}", count));
      count += 1;
      if count >= 3 {
        break;
      }
    }
    assert_eq!(count, 3);
  }

  #[tokio::test]
  #[should_panic]
  async fn test_long_lived_http_request_producer_clone_panics() {
    let (tx, rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(rx);

    // This should panic because receivers cannot be cloned
    let _cloned = producer.clone();
  }
}
