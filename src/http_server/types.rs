//! # HTTP Server Types
//!
//! Core types for representing HTTP requests and responses in the StreamWeave ecosystem.
//!
//! These types wrap Axum's request/response types with additional metadata and provide
//! a clean interface for working with HTTP in StreamWeave pipelines.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use axum::{
  body::Body,
  extract::Request,
  http::{HeaderMap, HeaderValue, Method, StatusCode, Uri},
};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use serde::{Deserialize, Serialize};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::collections::HashMap;

/// HTTP method enumeration for type-safe method handling.
///
/// This wraps Axum's `Method` type and provides additional convenience methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub enum HttpMethod {
  /// GET request
  Get,
  /// POST request
  Post,
  /// PUT request
  Put,
  /// DELETE request
  Delete,
  /// PATCH request
  Patch,
  /// HEAD request
  Head,
  /// OPTIONS request
  Options,
  /// TRACE request
  Trace,
  /// CONNECT request
  Connect,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl From<Method> for HttpMethod {
  fn from(method: Method) -> Self {
    match method {
      Method::GET => HttpMethod::Get,
      Method::POST => HttpMethod::Post,
      Method::PUT => HttpMethod::Put,
      Method::DELETE => HttpMethod::Delete,
      Method::PATCH => HttpMethod::Patch,
      Method::HEAD => HttpMethod::Head,
      Method::OPTIONS => HttpMethod::Options,
      Method::TRACE => HttpMethod::Trace,
      Method::CONNECT => HttpMethod::Connect,
      _ => HttpMethod::Get, // Default fallback
    }
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl From<HttpMethod> for Method {
  fn from(method: HttpMethod) -> Self {
    match method {
      HttpMethod::Get => Method::GET,
      HttpMethod::Post => Method::POST,
      HttpMethod::Put => Method::PUT,
      HttpMethod::Delete => Method::DELETE,
      HttpMethod::Patch => Method::PATCH,
      HttpMethod::Head => Method::HEAD,
      HttpMethod::Options => Method::OPTIONS,
      HttpMethod::Trace => Method::TRACE,
      HttpMethod::Connect => Method::CONNECT,
    }
  }
}

/// Content type enumeration for common HTTP content types.
///
/// This provides type-safe handling of common content types used in REST APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub enum ContentType {
  /// JSON content type (`application/json`)
  Json,
  /// Plain text content type (`text/plain`)
  Text,
  /// Binary/octet stream content type (`application/octet-stream`)
  Binary,
  /// HTML content type (`text/html`)
  Html,
  /// XML content type (`application/xml`)
  Xml,
  /// Form URL-encoded content type (`application/x-www-form-urlencoded`)
  FormUrlEncoded,
  /// Multipart form data (`multipart/form-data`)
  Multipart,
  /// Custom content type
  Custom(String),
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl ContentType {
  /// Get the MIME type string for this content type.
  pub fn as_str(&self) -> &str {
    match self {
      ContentType::Json => "application/json",
      ContentType::Text => "text/plain",
      ContentType::Binary => "application/octet-stream",
      ContentType::Html => "text/html",
      ContentType::Xml => "application/xml",
      ContentType::FormUrlEncoded => "application/x-www-form-urlencoded",
      ContentType::Multipart => "multipart/form-data",
      ContentType::Custom(s) => s,
    }
  }

  /// Parse a content type from a MIME type string.
  pub fn from_str(s: &str) -> Self {
    match s {
      "application/json" => ContentType::Json,
      "text/plain" => ContentType::Text,
      "application/octet-stream" => ContentType::Binary,
      "text/html" => ContentType::Html,
      "application/xml" => ContentType::Xml,
      "application/x-www-form-urlencoded" => ContentType::FormUrlEncoded,
      s if s.starts_with("multipart/form-data") => ContentType::Multipart,
      s => ContentType::Custom(s.to_string()),
    }
  }

  /// Get the content type from HTTP headers.
  pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
    headers
      .get("content-type")
      .and_then(|v| v.to_str().ok())
      .map(|s| {
        // Extract just the MIME type (before any parameters like charset)
        s.split(';').next().unwrap_or(s).trim()
      })
      .map(Self::from_str)
  }
}

/// HTTP request type wrapping Axum's request with additional metadata.
///
/// This type provides a clean interface for working with HTTP requests in StreamWeave pipelines.
/// It extracts and provides easy access to request metadata like method, path, headers, and query parameters.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::HttpRequest;
/// use axum::extract::Request;
///
/// async fn handle_request(axum_request: Request) -> HttpRequest {
///     HttpRequest::from_axum_request(axum_request).await
/// }
/// ```
#[derive(Debug, Clone)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub struct HttpRequest {
  /// HTTP method (GET, POST, etc.)
  pub method: HttpMethod,
  /// Request URI/path
  pub uri: Uri,
  /// Request path as string
  pub path: String,
  /// HTTP headers
  pub headers: HeaderMap,
  /// Query parameters parsed from the URI
  pub query_params: HashMap<String, String>,
  /// Path parameters (extracted from route patterns)
  pub path_params: HashMap<String, String>,
  /// Request body as bytes (if available)
  pub body: Option<Vec<u8>>,
  /// Content type of the request body
  pub content_type: Option<ContentType>,
  /// Remote address of the client
  pub remote_addr: Option<String>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl HttpRequest {
  /// Create an `HttpRequest` from an Axum `Request`.
  ///
  /// This extracts all relevant metadata from the Axum request and parses query parameters.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpRequest;
  /// use axum::extract::Request;
  ///
  /// async fn process_request(axum_request: Request) {
  ///     let request = HttpRequest::from_axum_request(axum_request).await;
  ///     println!("Method: {:?}, Path: {}", request.method, request.path);
  /// }
  /// ```
  pub async fn from_axum_request(mut axum_request: Request) -> Self {
    let method = HttpMethod::from(axum_request.method().clone());
    let uri = axum_request.uri().clone();
    let path = uri.path().to_string();
    let headers = axum_request.headers().clone();
    let content_type = ContentType::from_headers(&headers);

    // Parse query parameters
    let query_params = uri
      .query()
      .map(|q| serde_urlencoded::from_str::<HashMap<String, String>>(q).unwrap_or_default())
      .unwrap_or_default();

    // Extract remote address if available
    let remote_addr = axum_request
      .extensions()
      .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
      .map(|info| info.to_string());

    // Extract body if available
    let body = if axum_request.method() == &Method::GET || axum_request.method() == &Method::HEAD {
      None
    } else {
      // For now, we'll extract the body in the producer
      // This is a placeholder - actual body extraction happens in the producer
      None
    };

    Self {
      method,
      uri,
      path,
      headers,
      query_params,
      path_params: HashMap::new(), // Will be populated by route handlers
      body,
      content_type,
      remote_addr,
    }
  }

  /// Get a query parameter value by key.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpRequest;
  ///
  /// let request = /* ... */;
  /// if let Some(id) = request.get_query_param("id") {
  ///     println!("ID: {}", id);
  /// }
  /// ```
  pub fn get_query_param(&self, key: &str) -> Option<&String> {
    self.query_params.get(key)
  }

  /// Get a path parameter value by key.
  ///
  /// Path parameters are extracted from route patterns (e.g., `/users/:id`).
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpRequest;
  ///
  /// let request = /* ... */;
  /// if let Some(user_id) = request.get_path_param("id") {
  ///     println!("User ID: {}", user_id);
  /// }
  /// ```
  pub fn get_path_param(&self, key: &str) -> Option<&String> {
    self.path_params.get(key)
  }

  /// Get a header value by name.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpRequest;
  ///
  /// let request = /* ... */;
  /// if let Some(auth) = request.get_header("authorization") {
  ///     println!("Auth: {:?}", auth);
  /// }
  /// ```
  pub fn get_header(&self, name: &str) -> Option<&HeaderValue> {
    self.headers.get(name)
  }

  /// Check if the request has a specific content type.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::{HttpRequest, ContentType};
  ///
  /// let request = /* ... */;
  /// if request.is_content_type(ContentType::Json) {
  ///     // Handle JSON request
  /// }
  /// ```
  pub fn is_content_type(&self, content_type: ContentType) -> bool {
    self
      .content_type
      .as_ref()
      .map(|ct| *ct == content_type)
      .unwrap_or(false)
  }
}

/// HTTP response type for StreamWeave pipelines.
///
/// This type provides a clean interface for constructing HTTP responses from pipeline outputs.
/// It supports setting status codes, headers, and body content with different content types.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::{HttpResponse, ContentType, HttpMethod};
/// use axum::http::StatusCode;
///
/// let response = HttpResponse {
///     status: StatusCode::OK,
///     headers: HeaderMap::new(),
///     body: serde_json::to_vec(&data).unwrap(),
///     content_type: ContentType::Json,
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub struct HttpResponse {
  /// HTTP status code
  pub status: StatusCode,
  /// Response headers
  pub headers: HeaderMap,
  /// Response body as bytes
  pub body: Vec<u8>,
  /// Content type of the response body
  pub content_type: ContentType,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl HttpResponse {
  /// Create a new HTTP response with the given status code and body.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::{HttpResponse, ContentType};
  /// use axum::http::StatusCode;
  ///
  /// let response = HttpResponse::new(
  ///     StatusCode::OK,
  ///     b"Hello, world!".to_vec(),
  ///     ContentType::Text,
  /// );
  /// ```
  pub fn new(status: StatusCode, body: Vec<u8>, content_type: ContentType) -> Self {
    let mut headers = HeaderMap::new();
    headers.insert(
      "content-type",
      HeaderValue::from_str(content_type.as_str()).unwrap_or_default(),
    );

    Self {
      status,
      headers,
      body,
      content_type,
    }
  }

  /// Create a JSON response from a serializable value.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponse;
  /// use axum::http::StatusCode;
  /// use serde::Serialize;
  ///
  /// #[derive(Serialize)]
  /// struct Data { value: u32 }
  ///
  /// let data = Data { value: 42 };
  /// let response = HttpResponse::json(StatusCode::OK, &data).unwrap();
  /// ```
  pub fn json<T: Serialize>(status: StatusCode, value: &T) -> Result<Self, serde_json::Error> {
    let body = serde_json::to_vec(value)?;
    Ok(Self::new(status, body, ContentType::Json))
  }

  /// Create a text response.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponse;
  /// use axum::http::StatusCode;
  ///
  /// let response = HttpResponse::text(StatusCode::OK, "Hello, world!");
  /// ```
  pub fn text(status: StatusCode, text: &str) -> Self {
    Self::new(status, text.as_bytes().to_vec(), ContentType::Text)
  }

  /// Create a binary response.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponse;
  /// use axum::http::StatusCode;
  ///
  /// let data = vec![0u8, 1u8, 2u8];
  /// let response = HttpResponse::binary(StatusCode::OK, data);
  /// ```
  pub fn binary(status: StatusCode, body: Vec<u8>) -> Self {
    Self::new(status, body, ContentType::Binary)
  }

  /// Create an error response.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponse;
  /// use axum::http::StatusCode;
  ///
  /// let response = HttpResponse::error(
  ///     StatusCode::INTERNAL_SERVER_ERROR,
  ///     "An error occurred",
  /// );
  /// ```
  pub fn error(status: StatusCode, message: &str) -> Self {
    let error_json = serde_json::json!({
        "error": message,
        "status": status.as_u16(),
    });
    Self::json(status, &error_json).unwrap_or_else(|_| Self::text(status, message))
  }

  /// Add a header to the response.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponse;
  /// use axum::http::{StatusCode, HeaderValue};
  ///
  /// let mut response = HttpResponse::text(StatusCode::OK, "Hello");
  /// response.add_header("x-custom-header", HeaderValue::from_static("value"));
  /// ```
  pub fn add_header(&mut self, name: &str, value: HeaderValue) {
    self.headers.insert(name, value);
  }

  /// Set a header in the response (replaces existing value).
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponse;
  /// use axum::http::{StatusCode, HeaderValue};
  ///
  /// let mut response = HttpResponse::text(StatusCode::OK, "Hello");
  /// response.set_header("x-custom-header", HeaderValue::from_static("value"));
  /// ```
  pub fn set_header(&mut self, name: &str, value: HeaderValue) {
    self.headers.insert(name, value);
  }

  /// Convert the response to an Axum response.
  ///
  /// This converts the StreamWeave `HttpResponse` into an Axum `Response<Body>`.
  pub fn to_axum_response(self) -> axum::response::Response<Body> {
    let mut response = axum::response::Response::builder()
      .status(self.status)
      .body(Body::from(self.body))
      .unwrap();

    // Set headers
    *response.headers_mut() = self.headers;

    response
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Default for HttpResponse {
  fn default() -> Self {
    Self::new(StatusCode::OK, Vec::new(), ContentType::Text)
  }
}

/// Represents either an HTTP request with metadata or a body chunk when streaming.
///
/// This enum is used by `HttpRequestProducer` when streaming mode is enabled.
/// It allows the producer to yield the request metadata first, then stream
/// body chunks separately.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::{HttpRequestItem, HttpRequest};
///
/// // Request metadata
/// let request = HttpRequest::from_axum_request(axum_request).await;
/// let item = HttpRequestItem::Request(request);
///
/// // Body chunk
/// let chunk = vec![0u8, 1u8, 2u8];
/// let item = HttpRequestItem::BodyChunk(chunk);
/// ```
#[derive(Debug, Clone)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub enum HttpRequestItem {
  /// HTTP request with metadata (method, path, headers, etc.)
  /// When streaming, this is yielded first with body = None.
  Request(HttpRequest),
  /// A chunk of the request body (bytes).
  /// These are yielded after the Request item when streaming.
  BodyChunk(Vec<u8>),
}
