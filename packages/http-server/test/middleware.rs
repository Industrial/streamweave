#[cfg(test)]
mod tests {
  use axum::body::Body;
  use axum::extract::Request;
  use axum::http::HeaderValue;
  use axum::middleware::Next;
  use streamweave_http_server::middleware::{
    common_middleware_stack, cors_layer, cors_layer_with_origins, example_auth_middleware,
    logging_layer, rate_limit_layer,
  };

  #[test]
  fn test_cors_layer() {
    let layer = cors_layer();
    // Just verify it compiles and returns a CorsLayer
    // Full integration testing would require setting up an Axum server
    assert!(std::mem::size_of_val(&layer) > 0);
  }

  #[test]
  fn test_cors_layer_with_origins() {
    let origins = vec![
      HeaderValue::from_static("http://localhost:3000"),
      HeaderValue::from_static("https://example.com"),
    ];

    let layer = cors_layer_with_origins(origins);
    // Just verify it compiles and returns a CorsLayer
    assert!(std::mem::size_of_val(&layer) > 0);
  }

  #[test]
  fn test_logging_layer() {
    let layer = logging_layer();
    // Just verify it compiles and returns a TraceLayer
    assert!(std::mem::size_of_val(&layer) > 0);
  }

  #[tokio::test]
  async fn test_example_auth_middleware_with_token() {
    let request = Request::builder()
      .header("authorization", "Bearer test-token")
      .body(Body::empty())
      .unwrap();

    // Create a simple next function that returns OK
    let next = Next::new(|req: Request| {
      Box::pin(async move {
        axum::response::Response::builder()
          .status(axum::http::StatusCode::OK)
          .body(Body::empty())
          .unwrap()
      })
    });

    let response = example_auth_middleware(request, next).await;

    // Should pass through with valid token
    assert_eq!(response.status(), axum::http::StatusCode::OK);
  }

  #[tokio::test]
  async fn test_example_auth_middleware_without_token() {
    let request = Request::builder().body(Body::empty()).unwrap();

    let next = Next::new(|req: Request| {
      Box::pin(async move {
        axum::response::Response::builder()
          .status(axum::http::StatusCode::OK)
          .body(Body::empty())
          .unwrap()
      })
    });

    let response = example_auth_middleware(request, next).await;

    // Should return 401 without token
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn test_example_auth_middleware_invalid_token_format() {
    let request = Request::builder()
      .header("authorization", "Invalid token")
      .body(Body::empty())
      .unwrap();

    let next = Next::new(|req: Request| {
      Box::pin(async move {
        axum::response::Response::builder()
          .status(axum::http::StatusCode::OK)
          .body(Body::empty())
          .unwrap()
      })
    });

    let response = example_auth_middleware(request, next).await;

    // Should return 401 with invalid format
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
  }

  #[test]
  fn test_rate_limit_layer() {
    let layer = rate_limit_layer();
    // Just verify it compiles and returns an Identity layer
    assert!(std::mem::size_of_val(&layer) > 0);
  }

  #[test]
  fn test_common_middleware_stack() {
    let stack = common_middleware_stack();
    // Just verify it compiles and returns a middleware stack
    assert!(std::mem::size_of_val(&stack) > 0);
  }
}
