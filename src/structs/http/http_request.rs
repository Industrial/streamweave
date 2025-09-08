use bytes::Bytes;
use http::{HeaderMap, Method, Uri};
use std::collections::HashMap;

use crate::structs::http::connection_info::ConnectionInfo;

/// Enhanced HTTP request representation for StreamWeave
#[derive(Debug, Clone)]
pub struct StreamWeaveHttpRequest {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub body: Bytes,
  pub path_params: HashMap<String, String>,
  pub query_params: HashMap<String, String>,
  pub connection_info: ConnectionInfo,
}

impl StreamWeaveHttpRequest {
  pub fn new(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
    connection_info: ConnectionInfo,
  ) -> Self {
    let path_params = HashMap::new();
    let query_params = Self::parse_query_params(&uri);

    Self {
      method,
      uri,
      headers,
      body,
      path_params,
      query_params,
      connection_info,
    }
  }

  /// Parse query parameters from URI
  fn parse_query_params(uri: &Uri) -> HashMap<String, String> {
    uri
      .query()
      .map(|query| {
        url::form_urlencoded::parse(query.as_bytes())
          .into_owned()
          .collect()
      })
      .unwrap_or_default()
  }

  /// Create a StreamWeaveHttpRequest from an Axum Request
  pub async fn from_axum_request(req: axum::extract::Request) -> Self {
    let (parts, body) = req.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
      .await
      .unwrap_or_default();

    // Create a default connection info for producer usage
    let connection_info = ConnectionInfo::new(
      "127.0.0.1:8080".parse().unwrap(),
      "0.0.0.0:3000".parse().unwrap(),
      http::Version::HTTP_11,
    );

    Self::new(
      parts.method,
      parts.uri,
      parts.headers,
      body_bytes,
      connection_info,
    )
  }

  /// Get a path parameter by name
  pub fn path_param(&self, name: &str) -> Option<&String> {
    self.path_params.get(name)
  }

  /// Get a query parameter by name
  pub fn query_param(&self, name: &str) -> Option<&String> {
    self.query_params.get(name)
  }

  /// Set path parameters (used by router)
  pub fn set_path_params(&mut self, params: HashMap<String, String>) {
    self.path_params = params;
  }

  /// Get the path without query string
  pub fn path(&self) -> &str {
    self.uri.path()
  }

  /// Get the full URI as string
  pub fn uri_string(&self) -> String {
    self.uri.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use http::{Uri, Version};

  #[test]
  fn test_parse_query_params() {
    let uri: Uri = "http://example.com/path?param1=value1&param2=value2"
      .parse()
      .unwrap();
    let params = StreamWeaveHttpRequest::parse_query_params(&uri);

    assert_eq!(params.get("param1"), Some(&"value1".to_string()));
    assert_eq!(params.get("param2"), Some(&"value2".to_string()));
    assert_eq!(params.get("param3"), None);
  }

  #[test]
  fn test_parse_empty_query() {
    let uri: Uri = "http://example.com/path".parse().unwrap();
    let params = StreamWeaveHttpRequest::parse_query_params(&uri);

    assert!(params.is_empty());
  }

  #[test]
  fn test_path_param_access() {
    let mut request = StreamWeaveHttpRequest::new(
      Method::GET,
      "http://example.com/users/123".parse().unwrap(),
      HeaderMap::new(),
      Bytes::new(),
      ConnectionInfo::new(
        "127.0.0.1:8080".parse().unwrap(),
        "0.0.0.0:3000".parse().unwrap(),
        Version::HTTP_11,
      ),
    );

    let mut path_params = HashMap::new();
    path_params.insert("id".to_string(), "123".to_string());
    request.set_path_params(path_params);

    assert_eq!(request.path_param("id"), Some(&"123".to_string()));
    assert_eq!(request.path_param("name"), None);
  }

  #[test]
  fn test_query_param_access() {
    let request = StreamWeaveHttpRequest::new(
      Method::GET,
      "http://example.com/search?q=rust&page=1".parse().unwrap(),
      HeaderMap::new(),
      Bytes::new(),
      ConnectionInfo::new(
        "127.0.0.1:8080".parse().unwrap(),
        "0.0.0.0:3000".parse().unwrap(),
        Version::HTTP_11,
      ),
    );

    assert_eq!(request.query_param("q"), Some(&"rust".to_string()));
    assert_eq!(request.query_param("page"), Some(&"1".to_string()));
    assert_eq!(request.query_param("sort"), None);
  }
}
