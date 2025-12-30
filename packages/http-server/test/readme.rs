//! Integration tests for README examples
//!
//! These tests verify that the code examples in the README.md file compile and run correctly.

#[cfg(test)]
mod tests {
  // Note: Most HTTP server examples require an actual HTTP server to be running,
  // so these tests are primarily compile-time checks. Full integration tests
  // would require setting up an Axum server, which is beyond the scope of
  // simple README verification.

  #[test]
  fn test_readme_examples_compile() {
    // This test ensures that the README examples at least compile
    // Full integration testing would require HTTP server setup

    // Example: Path-based routing configuration
    use streamweave_http_server::path_router_transformer::{
      PathBasedRouterTransformer, PathRouterConfig, RoutePattern,
    };

    let config = PathRouterConfig {
      routes: vec![
        RoutePattern {
          pattern: "/api/rest/*".to_string(),
          port: 0,
        },
        RoutePattern {
          pattern: "/api/graphql".to_string(),
          port: 1,
        },
      ],
      default_port: Some(4),
    };

    let _router = PathBasedRouterTransformer::new(config);

    // Example: Request producer config
    use streamweave_http_server::producer::HttpRequestProducerConfig;

    let _producer_config = HttpRequestProducerConfig::default()
      .with_extract_body(true)
      .with_max_body_size(Some(10 * 1024 * 1024))
      .with_parse_json(true)
      .with_extract_query_params(true)
      .with_extract_path_params(true)
      .with_stream_body(false);

    // Example: Response consumer config
    use axum::http::StatusCode;
    use streamweave_http_server::consumer::HttpResponseConsumerConfig;

    let _consumer_config = HttpResponseConsumerConfig::default()
      .with_stream_response(false)
      .with_max_items(None)
      .with_merge_responses(false)
      .with_default_status(StatusCode::OK);
  }
}
