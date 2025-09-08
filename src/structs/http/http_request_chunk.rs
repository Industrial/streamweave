use bytes::Bytes;
use http::{HeaderMap, Method, Uri};
use std::collections::HashMap;

use crate::structs::http::connection_info::ConnectionInfo;

/// Streaming HTTP request chunk for processing
#[derive(Debug, Clone)]
pub struct StreamWeaveHttpRequestChunk {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub chunk: Bytes,
  pub path_params: HashMap<String, String>,
  pub query_params: HashMap<String, String>,
  pub connection_info: ConnectionInfo,
  pub is_final: bool,
}

impl StreamWeaveHttpRequestChunk {
  pub fn new(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    chunk: Bytes,
    connection_info: ConnectionInfo,
    is_final: bool,
  ) -> Self {
    let path_params = HashMap::new();
    let query_params = Self::parse_query_params(&uri);

    Self {
      method,
      uri,
      headers,
      chunk,
      path_params,
      query_params,
      connection_info,
      is_final,
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

  /// Check if this is the final chunk of the request
  pub fn is_final(&self) -> bool {
    self.is_final
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use http::Version;

  #[test]
  fn test_request_chunk() {
    let chunk = StreamWeaveHttpRequestChunk::new(
      Method::POST,
      "http://example.com/api/data".parse().unwrap(),
      HeaderMap::new(),
      Bytes::from("test data"),
      ConnectionInfo::new(
        "127.0.0.1:8080".parse().unwrap(),
        "0.0.0.0:3000".parse().unwrap(),
        Version::HTTP_11,
      ),
      false,
    );

    assert_eq!(chunk.method, Method::POST);
    assert_eq!(chunk.chunk, Bytes::from("test data"));
    assert!(!chunk.is_final());
  }
}
