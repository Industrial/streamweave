//! # HTTP Server Types
//!
//! Core types for representing HTTP requests and responses in the StreamWeave ecosystem.
//!
//! These types wrap Axum's request/response types with additional metadata and provide
//! a clean interface for working with HTTP in StreamWeave pipelines.

#[cfg(feature = "http-server")]
use axum::{
  body::Body,
  extract::Request,
  http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri},
};
#[cfg(feature = "http-server")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "http-server")]
use std::collections::HashMap;

/// Extension type for passing request ID through Axum request extensions.
/// Used by HttpGraphServer to ensure request_id consistency between
/// the handler and the producer.
#[derive(Clone, Debug)]
#[cfg(feature = "http-server")]
pub struct RequestIdExtension(pub String);

/// HTTP method enumeration for type-safe method handling.
///
/// This wraps Axum's `Method` type and provides additional convenience methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg(feature = "http-server")]
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

#[cfg(feature = "http-server")]
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

#[cfg(feature = "http-server")]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg(feature = "http-server")]
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

#[cfg(feature = "http-server")]
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
  #[allow(clippy::should_implement_trait)]
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
#[cfg(feature = "http-server")]
pub struct HttpRequest {
  /// Unique request ID for correlation with responses
  pub request_id: String,
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

#[cfg(feature = "http-server")]
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
  pub async fn from_axum_request(axum_request: Request) -> Self {
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
    let body = if axum_request.method() == Method::GET || axum_request.method() == Method::HEAD {
      None
    } else {
      // For now, we'll extract the body in the producer
      // This is a placeholder - actual body extraction happens in the producer
      None
    };

    // Generate a unique request ID
    // Check if a request_id was provided in extensions (for graph server integration)
    let request_id = axum_request
      .extensions()
      .get::<RequestIdExtension>()
      .map(|ext| ext.0.clone())
      .unwrap_or_else(|| {
        // For now, use a simple counter-based ID. In production, consider using UUIDs.
        format!(
          "req-{}",
          std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
        )
      });

    Self {
      request_id,
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

  /// Check if this HttpRequest is a body chunk (from streaming mode).
  ///
  /// When streaming is enabled, the producer yields:
  /// 1. First: Request metadata with body = None
  /// 2. Then: Body chunks with body = Some(chunk) and progress headers
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpRequest;
  ///
  /// let request = /* ... */;
  /// if request.is_body_chunk() {
  ///     let offset = request.get_chunk_offset().unwrap_or(0);
  ///     let size = request.get_chunk_size().unwrap_or(0);
  ///     println!("Processing chunk at offset {} with size {}", offset, size);
  /// }
  /// ```
  pub fn is_body_chunk(&self) -> bool {
    // A body chunk has a body and progress tracking headers
    self.body.is_some() && self.headers.contains_key("x-streamweave-chunk-offset")
  }

  /// Get the byte offset of this body chunk (for progress tracking).
  ///
  /// Returns None if this is not a body chunk or if the offset header is missing.
  pub fn get_chunk_offset(&self) -> Option<u64> {
    self
      .headers
      .get("x-streamweave-chunk-offset")
      .and_then(|v| v.to_str().ok())
      .and_then(|s| s.parse::<u64>().ok())
  }

  /// Get the size of this body chunk in bytes.
  ///
  /// Returns None if this is not a body chunk or if the size header is missing.
  pub fn get_chunk_size(&self) -> Option<usize> {
    self
      .headers
      .get("x-streamweave-chunk-size")
      .and_then(|v| v.to_str().ok())
      .and_then(|s| s.parse::<usize>().ok())
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
#[cfg(feature = "http-server")]
pub struct HttpResponse {
  /// Request ID for correlation with the original request
  pub request_id: String,
  /// HTTP status code
  pub status: StatusCode,
  /// Response headers
  pub headers: HeaderMap,
  /// Response body as bytes
  pub body: Vec<u8>,
  /// Content type of the response body
  pub content_type: ContentType,
}

#[cfg(feature = "http-server")]
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
    Self::with_request_id(status, body, content_type, String::new())
  }

  /// Create a new HTTP response with the given status code, body, and request ID.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::{HttpResponse, ContentType};
  /// use axum::http::StatusCode;
  ///
  /// let response = HttpResponse::with_request_id(
  ///     StatusCode::OK,
  ///     b"Hello, world!".to_vec(),
  ///     ContentType::Text,
  ///     "req-123".to_string(),
  /// );
  /// ```
  pub fn with_request_id(
    status: StatusCode,
    body: Vec<u8>,
    content_type: ContentType,
    request_id: String,
  ) -> Self {
    let mut headers = HeaderMap::new();
    let content_type_value = HeaderValue::from_str(content_type.as_str())
      .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    headers.insert("content-type", content_type_value);

    Self {
      request_id,
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
    Self::json_with_request_id(status, value, String::new())
  }

  /// Create a JSON response from a serializable value with a request ID.
  pub fn json_with_request_id<T: Serialize>(
    status: StatusCode,
    value: &T,
    request_id: String,
  ) -> Result<Self, serde_json::Error> {
    let body = serde_json::to_vec(value)?;
    Ok(Self::with_request_id(
      status,
      body,
      ContentType::Json,
      request_id,
    ))
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
    Self::text_with_request_id(status, text, String::new())
  }

  /// Create a text response with a request ID.
  pub fn text_with_request_id(status: StatusCode, text: &str, request_id: String) -> Self {
    Self::with_request_id(
      status,
      text.as_bytes().to_vec(),
      ContentType::Text,
      request_id,
    )
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
    Self::binary_with_request_id(status, body, String::new())
  }

  /// Create a binary response with a request ID.
  pub fn binary_with_request_id(status: StatusCode, body: Vec<u8>, request_id: String) -> Self {
    Self::with_request_id(status, body, ContentType::Binary, request_id)
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
    Self::error_with_request_id(status, message, String::new())
  }

  /// Create an error response with a request ID.
  pub fn error_with_request_id(status: StatusCode, message: &str, request_id: String) -> Self {
    let error_json = serde_json::json!({
        "error": message,
        "status": status.as_u16(),
    });
    Self::json_with_request_id(status, &error_json, request_id)
      .unwrap_or_else(|_| Self::text_with_request_id(status, message, String::new()))
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
    if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) {
      self.headers.insert(header_name, value);
    }
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
    if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) {
      self.headers.insert(header_name, value);
    }
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

#[cfg(feature = "http-server")]
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
#[cfg(feature = "http-server")]
#[allow(clippy::large_enum_variant)]
pub enum HttpRequestItem {
  /// HTTP request with metadata (method, path, headers, etc.)
  /// When streaming, this is yielded first with body = None.
  Request(HttpRequest),
  /// A chunk of the request body (bytes).
  /// These are yielded after the Request item when streaming.
  BodyChunk(Vec<u8>),
}
