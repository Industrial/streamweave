#[cfg(test)]
mod tests {
  use futures::stream;
  use std::collections::HashMap;
  use streamweave::message::{Message, MessageId};
  use streamweave_http_server::path_router_transformer::{
    PathBasedRouterTransformer, PathRouterConfig, RoutePattern,
  };
  use streamweave_http_server::types::HttpRequest;

  #[tokio::test]
  async fn test_path_matching() {
    let router = PathBasedRouterTransformer::new(PathRouterConfig::default());

    // Test exact match
    assert!(router.path_matches("/api/graphql", "/api/graphql"));
    assert!(!router.path_matches("/api/graphql", "/api/rest"));

    // Test wildcard match
    assert!(router.path_matches("/api/rest/*", "/api/rest/users"));
    assert!(router.path_matches("/api/rest/*", "/api/rest/users/123"));
    assert!(!router.path_matches("/api/rest/*", "/api/graphql"));
  }

  #[tokio::test]
  async fn test_route_matching() {
    // Create router with some routes
    let router = PathBasedRouterTransformer::new(PathRouterConfig {
      routes: vec![
        RoutePattern {
          pattern: "/api/rest/*".to_string(),
          port: 0,
        },
        RoutePattern {
          pattern: "/api/graphql".to_string(),
          port: 1,
        },
        RoutePattern {
          pattern: "/api/rpc/*".to_string(),
          port: 2,
        },
        RoutePattern {
          pattern: "/static/*".to_string(),
          port: 3,
        },
      ],
      default_port: Some(4),
    });

    assert_eq!(router.match_route("/api/rest/users"), Some(0));
    assert_eq!(router.match_route("/api/graphql"), Some(1));
    assert_eq!(router.match_route("/api/rpc/call"), Some(2));
    assert_eq!(router.match_route("/static/file.css"), Some(3));
    assert_eq!(router.match_route("/unknown/path"), Some(4)); // Default

    // Test empty router - should return None for unmatched paths
    let empty_router = PathBasedRouterTransformer::new(PathRouterConfig::default());
    assert_eq!(empty_router.match_route("/any/path"), None);
  }

  #[tokio::test]
  async fn test_routing_transformation() {
    let mut router = PathBasedRouterTransformer::new(PathRouterConfig {
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
      default_port: None,
    });

    let request1 = HttpRequest {
      request_id: "req1".to_string(),
      method: streamweave_http_server::types::HttpMethod::Get,
      uri: axum::http::Uri::from_static("/api/rest/users"),
      path: "/api/rest/users".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let request2 = HttpRequest {
      request_id: "req2".to_string(),
      method: streamweave_http_server::types::HttpMethod::Post,
      uri: axum::http::Uri::from_static("/api/graphql"),
      path: "/api/graphql".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let msg1 = Message::new(request1, MessageId::new_custom("msg1"));
    let msg2 = Message::new(request2, MessageId::new_custom("msg2"));

    let input = stream::iter(vec![msg1, msg2]);
    let boxed_input = Box::pin(input);

    let results: Vec<_> = router.transform(boxed_input).await.collect().await;

    // First request should go to port 0 (REST)
    assert!(results[0].0.is_some());
    assert!(results[0].1.is_none());
    assert!(results[0].2.is_none());
    assert!(results[0].3.is_none());
    assert!(results[0].4.is_none());

    // Second request should go to port 1 (GraphQL)
    assert!(results[1].0.is_none());
    assert!(results[1].1.is_some());
    assert!(results[1].2.is_none());
    assert!(results[1].3.is_none());
    assert!(results[1].4.is_none());
  }

  #[tokio::test]
  async fn test_path_router_add_route() {
    let mut router = PathBasedRouterTransformer::new(PathRouterConfig::default());
    router.add_route("/api/test".to_string(), 0);

    assert_eq!(router.router_config().routes.len(), 1);
    assert_eq!(router.router_config().routes[0].pattern, "/api/test");
    assert_eq!(router.router_config().routes[0].port, 0);
  }

  #[tokio::test]
  async fn test_path_router_set_default_port() {
    let mut router = PathBasedRouterTransformer::new(PathRouterConfig::default());
    router.set_default_port(Some(4));

    assert_eq!(router.router_config().default_port, Some(4));
  }

  #[tokio::test]
  async fn test_path_router_router_config() {
    let router = PathBasedRouterTransformer::new(PathRouterConfig {
      routes: vec![RoutePattern {
        pattern: "/api/*".to_string(),
        port: 0,
      }],
      default_port: Some(1),
    });

    let config = router.router_config();
    assert_eq!(config.routes.len(), 1);
    assert_eq!(config.default_port, Some(1));
  }

  #[tokio::test]
  async fn test_path_router_router_config_mut() {
    let mut router = PathBasedRouterTransformer::new(PathRouterConfig::default());
    router.router_config_mut().default_port = Some(2);

    assert_eq!(router.router_config().default_port, Some(2));
  }

  #[tokio::test]
  async fn test_path_router_with_routes() {
    let routes = vec![
      RoutePattern {
        pattern: "/api/*".to_string(),
        port: 0,
      },
      RoutePattern {
        pattern: "/static/*".to_string(),
        port: 1,
      },
    ];

    let router = PathBasedRouterTransformer::with_routes(routes, Some(2));

    assert_eq!(router.router_config().routes.len(), 2);
    assert_eq!(router.router_config().default_port, Some(2));
  }

  #[tokio::test]
  async fn test_path_router_unmatched_without_default() {
    let router = PathBasedRouterTransformer::new(PathRouterConfig {
      routes: vec![RoutePattern {
        pattern: "/api/*".to_string(),
        port: 0,
      }],
      default_port: None,
    });

    let request = HttpRequest {
      request_id: "req1".to_string(),
      method: streamweave_http_server::types::HttpMethod::Get,
      uri: axum::http::Uri::from_static("/unknown/path"),
      path: "/unknown/path".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let msg = Message::new(request, MessageId::new_custom("msg1"));
    let input = stream::iter(vec![msg]);
    let boxed_input = Box::pin(input);

    let results: Vec<_> = router.transform(boxed_input).await.collect().await;

    // Should drop the request (all ports None)
    assert!(results[0].0.is_none());
    assert!(results[0].1.is_none());
    assert!(results[0].2.is_none());
    assert!(results[0].3.is_none());
    assert!(results[0].4.is_none());
  }

  #[tokio::test]
  async fn test_path_router_port_2_3_4() {
    let mut router = PathBasedRouterTransformer::new(PathRouterConfig::default());
    router.add_route("/rpc/*".to_string(), 2);
    router.add_route("/static/*".to_string(), 3);
    router.set_default_port(Some(4));

    let request1 = HttpRequest {
      request_id: "req1".to_string(),
      method: streamweave_http_server::types::HttpMethod::Get,
      uri: axum::http::Uri::from_static("/rpc/call"),
      path: "/rpc/call".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let request2 = HttpRequest {
      request_id: "req2".to_string(),
      method: streamweave_http_server::types::HttpMethod::Get,
      uri: axum::http::Uri::from_static("/static/file.css"),
      path: "/static/file.css".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let request3 = HttpRequest {
      request_id: "req3".to_string(),
      method: streamweave_http_server::types::HttpMethod::Get,
      uri: axum::http::Uri::from_static("/unknown"),
      path: "/unknown".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let msg1 = Message::new(request1, MessageId::new_custom("msg1"));
    let msg2 = Message::new(request2, MessageId::new_custom("msg2"));
    let msg3 = Message::new(request3, MessageId::new_custom("msg3"));

    let input = stream::iter(vec![msg1, msg2, msg3]);
    let boxed_input = Box::pin(input);

    let results: Vec<_> = router.transform(boxed_input).await.collect().await;

    // First request should go to port 2 (RPC)
    assert!(results[0].2.is_some());
    assert!(results[0].0.is_none());
    assert!(results[0].1.is_none());
    assert!(results[0].3.is_none());
    assert!(results[0].4.is_none());

    // Second request should go to port 3 (Static)
    assert!(results[1].3.is_some());
    assert!(results[1].0.is_none());
    assert!(results[1].1.is_none());
    assert!(results[1].2.is_none());
    assert!(results[1].4.is_none());

    // Third request should go to port 4 (Default)
    assert!(results[2].4.is_some());
    assert!(results[2].0.is_none());
    assert!(results[2].1.is_none());
    assert!(results[2].2.is_none());
    assert!(results[2].3.is_none());
  }

  #[tokio::test]
  async fn test_path_router_wildcard_edge_cases() {
    let router = PathBasedRouterTransformer::new(PathRouterConfig::default());

    // Test various wildcard patterns
    assert!(router.path_matches("/api/*", "/api/"));
    assert!(router.path_matches("/api/*", "/api/users"));
    assert!(router.path_matches("/api/*", "/api/users/123"));
    assert!(!router.path_matches("/api/*", "/api"));
    assert!(!router.path_matches("/api/*", "/other"));
  }

  #[tokio::test]
  async fn test_path_router_exact_match_edge_cases() {
    let router = PathBasedRouterTransformer::new(PathRouterConfig::default());

    // Test exact matches
    assert!(router.path_matches("/", "/"));
    assert!(router.path_matches("/api", "/api"));
    assert!(!router.path_matches("/api", "/api/"));
    assert!(!router.path_matches("/api/", "/api"));
  }
}
