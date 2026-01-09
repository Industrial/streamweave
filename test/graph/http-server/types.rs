#[cfg(test)]
mod tests {
  use axum::body::Body;
  use axum::extract::Request;
  use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
  use std::collections::HashMap;
  use streamweave_http_server::types::{
    ContentType, HttpMethod, HttpRequest, HttpResponse, RequestIdExtension,
  };

  #[test]
  fn test_http_method_from_method() {
    assert_eq!(HttpMethod::from(Method::GET), HttpMethod::Get);
    assert_eq!(HttpMethod::from(Method::POST), HttpMethod::Post);
    assert_eq!(HttpMethod::from(Method::PUT), HttpMethod::Put);
    assert_eq!(HttpMethod::from(Method::DELETE), HttpMethod::Delete);
    assert_eq!(HttpMethod::from(Method::PATCH), HttpMethod::Patch);
    assert_eq!(HttpMethod::from(Method::HEAD), HttpMethod::Head);
    assert_eq!(HttpMethod::from(Method::OPTIONS), HttpMethod::Options);
    assert_eq!(HttpMethod::from(Method::TRACE), HttpMethod::Trace);
    assert_eq!(HttpMethod::from(Method::CONNECT), HttpMethod::Connect);
  }

  #[test]
  fn test_http_method_into_method() {
    assert_eq!(Method::from(HttpMethod::Get), Method::GET);
    assert_eq!(Method::from(HttpMethod::Post), Method::POST);
    assert_eq!(Method::from(HttpMethod::Put), Method::PUT);
    assert_eq!(Method::from(HttpMethod::Delete), Method::DELETE);
    assert_eq!(Method::from(HttpMethod::Patch), Method::PATCH);
    assert_eq!(Method::from(HttpMethod::Head), Method::HEAD);
    assert_eq!(Method::from(HttpMethod::Options), Method::OPTIONS);
    assert_eq!(Method::from(HttpMethod::Trace), Method::TRACE);
    assert_eq!(Method::from(HttpMethod::Connect), Method::CONNECT);
  }

  #[test]
  fn test_content_type_as_str() {
    assert_eq!(ContentType::Json.as_str(), "application/json");
    assert_eq!(ContentType::Text.as_str(), "text/plain");
    assert_eq!(ContentType::Binary.as_str(), "application/octet-stream");
    assert_eq!(ContentType::Html.as_str(), "text/html");
    assert_eq!(ContentType::Xml.as_str(), "application/xml");
    assert_eq!(
      ContentType::FormUrlEncoded.as_str(),
      "application/x-www-form-urlencoded"
    );
    assert_eq!(ContentType::Multipart.as_str(), "multipart/form-data");
    assert_eq!(
      ContentType::Custom("custom/type".to_string()).as_str(),
      "custom/type"
    );
  }

  #[test]
  fn test_content_type_from_str() {
    assert_eq!(ContentType::from_str("application/json"), ContentType::Json);
    assert_eq!(ContentType::from_str("text/plain"), ContentType::Text);
    assert_eq!(
      ContentType::from_str("application/octet-stream"),
      ContentType::Binary
    );
    assert_eq!(ContentType::from_str("text/html"), ContentType::Html);
    assert_eq!(ContentType::from_str("application/xml"), ContentType::Xml);
    assert_eq!(
      ContentType::from_str("application/x-www-form-urlencoded"),
      ContentType::FormUrlEncoded
    );
    assert_eq!(
      ContentType::from_str("multipart/form-data; boundary=something"),
      ContentType::Multipart
    );
    assert_eq!(
      ContentType::from_str("custom/type"),
      ContentType::Custom("custom/type".to_string())
    );
  }

  #[test]
  fn test_content_type_from_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("application/json"));

    let content_type = ContentType::from_headers(&headers);
    assert_eq!(content_type, Some(ContentType::Json));
  }

  #[test]
  fn test_content_type_from_headers_with_charset() {
    let mut headers = HeaderMap::new();
    headers.insert(
      "content-type",
      HeaderValue::from_static("application/json; charset=utf-8"),
    );

    let content_type = ContentType::from_headers(&headers);
    assert_eq!(content_type, Some(ContentType::Json));
  }

  #[test]
  fn test_content_type_from_headers_missing() {
    let headers = HeaderMap::new();
    let content_type = ContentType::from_headers(&headers);
    assert_eq!(content_type, None);
  }

  #[tokio::test]
  async fn test_http_request_from_axum_request() {
    let request = Request::builder()
      .uri("/test?key=value")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let http_request = HttpRequest::from_axum_request(request).await;

    assert_eq!(http_request.method, HttpMethod::Get);
    assert_eq!(http_request.path, "/test");
    assert_eq!(
      http_request.get_query_param("key"),
      Some(&"value".to_string())
    );
  }

  #[tokio::test]
  async fn test_http_request_get_query_param() {
    let request = Request::builder()
      .uri("/test?foo=bar&baz=qux")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let http_request = HttpRequest::from_axum_request(request).await;

    assert_eq!(
      http_request.get_query_param("foo"),
      Some(&"bar".to_string())
    );
    assert_eq!(
      http_request.get_query_param("baz"),
      Some(&"qux".to_string())
    );
    assert_eq!(http_request.get_query_param("nonexistent"), None);
  }

  #[tokio::test]
  async fn test_http_request_get_path_param() {
    let mut request = HttpRequest {
      request_id: "test".to_string(),
      method: HttpMethod::Get,
      uri: Uri::from_static("/users/123"),
      path: "/users/123".to_string(),
      headers: HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: {
        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());
        params
      },
      body: None,
      content_type: None,
      remote_addr: None,
    };

    assert_eq!(request.get_path_param("id"), Some(&"123".to_string()));
    assert_eq!(request.get_path_param("nonexistent"), None);
  }

  #[tokio::test]
  async fn test_http_request_get_header() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", HeaderValue::from_static("Bearer token"));

    let request = HttpRequest {
      request_id: "test".to_string(),
      method: HttpMethod::Get,
      uri: Uri::from_static("/test"),
      path: "/test".to_string(),
      headers,
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    assert_eq!(
      request.get_header("authorization"),
      Some(&HeaderValue::from_static("Bearer token"))
    );
    assert_eq!(request.get_header("nonexistent"), None);
  }

  #[tokio::test]
  async fn test_http_request_is_content_type() {
    let request = HttpRequest {
      request_id: "test".to_string(),
      method: HttpMethod::Post,
      uri: Uri::from_static("/test"),
      path: "/test".to_string(),
      headers: {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers
      },
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: Some(ContentType::Json),
      remote_addr: None,
    };

    assert!(request.is_content_type(ContentType::Json));
    assert!(!request.is_content_type(ContentType::Text));
  }

  #[tokio::test]
  async fn test_http_request_is_body_chunk() {
    let mut headers = HeaderMap::new();
    headers.insert("x-streamweave-chunk-offset", HeaderValue::from_static("0"));
    headers.insert("x-streamweave-chunk-size", HeaderValue::from_static("100"));

    let request = HttpRequest {
      request_id: "test".to_string(),
      method: HttpMethod::Post,
      uri: Uri::from_static("/test"),
      path: "/test".to_string(),
      headers,
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: Some(vec![1, 2, 3]),
      content_type: None,
      remote_addr: None,
    };

    assert!(request.is_body_chunk());
  }

  #[tokio::test]
  async fn test_http_request_get_chunk_offset() {
    let mut headers = HeaderMap::new();
    headers.insert("x-streamweave-chunk-offset", HeaderValue::from_static("42"));

    let request = HttpRequest {
      request_id: "test".to_string(),
      method: HttpMethod::Post,
      uri: Uri::from_static("/test"),
      path: "/test".to_string(),
      headers,
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: Some(vec![1, 2, 3]),
      content_type: None,
      remote_addr: None,
    };

    assert_eq!(request.get_chunk_offset(), Some(42));
  }

  #[tokio::test]
  async fn test_http_request_get_chunk_size() {
    let mut headers = HeaderMap::new();
    headers.insert("x-streamweave-chunk-size", HeaderValue::from_static("100"));

    let request = HttpRequest {
      request_id: "test".to_string(),
      method: HttpMethod::Post,
      uri: Uri::from_static("/test"),
      path: "/test".to_string(),
      headers,
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: Some(vec![1, 2, 3]),
      content_type: None,
      remote_addr: None,
    };

    assert_eq!(request.get_chunk_size(), Some(100));
  }

  #[test]
  fn test_http_response_new() {
    let response = HttpResponse::new(StatusCode::OK, b"Hello".to_vec(), ContentType::Text);

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, b"Hello".to_vec());
    assert_eq!(response.content_type, ContentType::Text);
  }

  #[test]
  fn test_http_response_with_request_id() {
    let response = HttpResponse::with_request_id(
      StatusCode::OK,
      b"Hello".to_vec(),
      ContentType::Text,
      "req-123".to_string(),
    );

    assert_eq!(response.request_id, "req-123");
    assert_eq!(response.status, StatusCode::OK);
  }

  #[test]
  fn test_http_response_json() {
    #[derive(serde::Serialize)]
    struct Data {
      value: u32,
    }

    let data = Data { value: 42 };
    let response = HttpResponse::json(StatusCode::OK, &data).unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.content_type, ContentType::Json);
    assert!(response.body.len() > 0);
  }

  #[test]
  fn test_http_response_json_with_request_id() {
    #[derive(serde::Serialize)]
    struct Data {
      value: u32,
    }

    let data = Data { value: 42 };
    let response =
      HttpResponse::json_with_request_id(StatusCode::OK, &data, "req-123".to_string()).unwrap();

    assert_eq!(response.request_id, "req-123");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.content_type, ContentType::Json);
  }

  #[test]
  fn test_http_response_text() {
    let response = HttpResponse::text(StatusCode::OK, "Hello, world!");

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.content_type, ContentType::Text);
    assert_eq!(response.body, b"Hello, world!".to_vec());
  }

  #[test]
  fn test_http_response_text_with_request_id() {
    let response =
      HttpResponse::text_with_request_id(StatusCode::OK, "Hello", "req-123".to_string());

    assert_eq!(response.request_id, "req-123");
    assert_eq!(response.status, StatusCode::OK);
  }

  #[test]
  fn test_http_response_binary() {
    let data = vec![1, 2, 3, 4, 5];
    let response = HttpResponse::binary(StatusCode::OK, data.clone());

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.content_type, ContentType::Binary);
    assert_eq!(response.body, data);
  }

  #[test]
  fn test_http_response_binary_with_request_id() {
    let data = vec![1, 2, 3];
    let response =
      HttpResponse::binary_with_request_id(StatusCode::OK, data, "req-123".to_string());

    assert_eq!(response.request_id, "req-123");
    assert_eq!(response.status, StatusCode::OK);
  }

  #[test]
  fn test_http_response_error() {
    let response = HttpResponse::error(StatusCode::BAD_REQUEST, "Invalid input");

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.content_type, ContentType::Json);
    assert!(response.body.len() > 0);
  }

  #[test]
  fn test_http_response_error_with_request_id() {
    let response = HttpResponse::error_with_request_id(
      StatusCode::BAD_REQUEST,
      "Invalid input",
      "req-123".to_string(),
    );

    assert_eq!(response.request_id, "req-123");
    assert_eq!(response.status, StatusCode::BAD_REQUEST);
  }

  #[test]
  fn test_http_response_add_header() {
    let mut response = HttpResponse::text(StatusCode::OK, "Hello");
    response.add_header("x-custom", HeaderValue::from_static("value"));

    assert!(response.headers.contains_key("x-custom"));
  }

  #[test]
  fn test_http_response_set_header() {
    let mut response = HttpResponse::text(StatusCode::OK, "Hello");
    response.set_header("x-custom", HeaderValue::from_static("value"));

    assert_eq!(
      response.headers.get("x-custom"),
      Some(&HeaderValue::from_static("value"))
    );
  }

  #[test]
  fn test_http_response_to_axum_response() {
    let response = HttpResponse::text(StatusCode::OK, "Hello");
    let axum_response = response.to_axum_response();

    assert_eq!(axum_response.status(), StatusCode::OK);
  }

  #[test]
  fn test_http_response_default() {
    let response = HttpResponse::default();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.content_type, ContentType::Text);
    assert_eq!(response.body, Vec::<u8>::new());
  }

  #[test]
  fn test_request_id_extension() {
    let ext = RequestIdExtension("req-123".to_string());
    assert_eq!(ext.0, "req-123");
  }
}
