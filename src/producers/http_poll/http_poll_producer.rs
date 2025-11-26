use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;

/// Configuration for HTTP polling producer behavior.
#[derive(Debug, Clone)]
pub struct HttpPollProducerConfig {
  /// HTTP endpoint URL to poll.
  pub url: String,
  /// HTTP method (GET, POST, etc.).
  pub method: String,
  /// HTTP headers to include in requests.
  pub headers: HashMap<String, String>,
  /// Request body (for POST/PUT requests).
  pub body: Option<String>,
  /// Polling interval between requests.
  pub poll_interval: Duration,
  /// Maximum number of requests before stopping (None for infinite).
  pub max_requests: Option<usize>,
  /// Rate limiting: maximum requests per second.
  pub rate_limit: Option<f64>,
  /// Pagination configuration.
  pub pagination: Option<PaginationConfig>,
  /// Delta detection configuration.
  pub delta_detection: Option<DeltaDetectionConfig>,
  /// Request timeout.
  pub timeout: Duration,
  /// Number of retries on failure.
  pub retries: u32,
  /// Retry backoff strategy.
  pub retry_backoff: Duration,
}

impl Default for HttpPollProducerConfig {
  fn default() -> Self {
    Self {
      url: String::new(),
      method: "GET".to_string(),
      headers: HashMap::new(),
      body: None,
      poll_interval: Duration::from_secs(60),
      max_requests: None,
      rate_limit: None,
      pagination: None,
      delta_detection: None,
      timeout: Duration::from_secs(30),
      retries: 3,
      retry_backoff: Duration::from_secs(1),
    }
  }
}

impl HttpPollProducerConfig {
  /// Sets the URL to poll.
  #[must_use]
  pub fn with_url(mut self, url: impl Into<String>) -> Self {
    self.url = url.into();
    self
  }

  /// Sets the HTTP method.
  #[must_use]
  pub fn with_method(mut self, method: impl Into<String>) -> Self {
    self.method = method.into();
    self
  }

  /// Sets the poll interval.
  #[must_use]
  pub fn with_poll_interval(mut self, interval: Duration) -> Self {
    self.poll_interval = interval;
    self
  }

  /// Sets the maximum number of requests.
  #[must_use]
  pub fn with_max_requests(mut self, max: usize) -> Self {
    self.max_requests = Some(max);
    self
  }

  /// Sets the rate limit (requests per second).
  #[must_use]
  pub fn with_rate_limit(mut self, rps: f64) -> Self {
    self.rate_limit = Some(rps);
    self
  }

  /// Sets pagination configuration.
  #[must_use]
  pub fn with_pagination(mut self, pagination: PaginationConfig) -> Self {
    self.pagination = Some(pagination);
    self
  }

  /// Sets delta detection configuration.
  #[must_use]
  pub fn with_delta_detection(mut self, delta: DeltaDetectionConfig) -> Self {
    self.delta_detection = Some(delta);
    self
  }

  /// Adds an HTTP header.
  pub fn add_header(
    &mut self,
    key: &str,
    value: &str,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    self.headers.insert(key.to_string(), value.to_string());
    Ok(())
  }

  /// Sets the request body.
  #[must_use]
  pub fn with_body(mut self, body: impl Into<String>) -> Self {
    self.body = Some(body.into());
    self
  }
}

/// Configuration for pagination handling.
#[derive(Debug, Clone)]
pub enum PaginationConfig {
  /// Query parameter-based pagination (e.g., ?page=1, ?offset=0, ?limit=100).
  QueryParameter {
    /// Parameter name for page/offset (e.g., "page", "offset").
    page_param: String,
    /// Parameter name for limit/size (optional, e.g., "limit", "size").
    limit_param: Option<String>,
    /// Page/offset increment step.
    step: usize,
    /// Initial page/offset value.
    start: usize,
    /// Maximum number of pages to fetch (None for all).
    max_pages: Option<usize>,
  },
  /// Header-based pagination (e.g., Link header with rel="next").
  LinkHeader,
  /// JSON response field-based pagination (e.g., { "next": "...", "data": [...] }).
  JsonField {
    /// Field name containing the next page URL.
    next_field: String,
    /// Field name containing the data array.
    data_field: String,
  },
}

/// Configuration for delta detection (only emit new items).
#[derive(Debug, Clone)]
pub struct DeltaDetectionConfig {
  /// Field name to use as unique identifier for items.
  pub id_field: String,
  /// Set of previously seen IDs (maintained by producer).
  pub seen_ids: HashSet<String>,
}

impl DeltaDetectionConfig {
  /// Creates a new delta detection configuration.
  #[must_use]
  pub fn new(id_field: impl Into<String>) -> Self {
    Self {
      id_field: id_field.into(),
      seen_ids: HashSet::new(),
    }
  }

  /// Creates delta detection with existing seen IDs.
  #[must_use]
  pub fn with_seen_ids(id_field: impl Into<String>, seen_ids: HashSet<String>) -> Self {
    Self {
      id_field: id_field.into(),
      seen_ids,
    }
  }
}

/// A response from an HTTP poll request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpPollResponse {
  /// Response status code.
  pub status: u16,
  /// Response headers.
  pub headers: HashMap<String, String>,
  /// Response body as JSON.
  pub body: serde_json::Value,
  /// Request URL that was polled.
  pub url: String,
}

impl HttpPollResponse {
  /// Creates a new HTTP poll response.
  #[must_use]
  pub fn new(
    status: u16,
    headers: HashMap<String, String>,
    body: serde_json::Value,
    url: String,
  ) -> Self {
    Self {
      status,
      headers,
      body,
      url,
    }
  }

  /// Converts a HeaderMap to HashMap<String, String>.
  #[must_use]
  pub fn headers_from_reqwest(headers: &reqwest::header::HeaderMap) -> HashMap<String, String> {
    headers
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
      .collect()
  }

  /// Converts a HashMap<String, String> to HeaderMap.
  #[must_use]
  pub fn headers_to_reqwest(headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
    let mut header_map = reqwest::header::HeaderMap::new();
    for (k, v) in headers {
      if let (Ok(name), Ok(value)) = (
        reqwest::header::HeaderName::from_bytes(k.as_bytes()),
        reqwest::header::HeaderValue::from_str(v),
      ) {
        header_map.insert(name, value);
      }
    }
    header_map
  }
}

/// A producer that polls HTTP endpoints at configurable intervals.
///
/// This producer supports:
/// - Configurable polling intervals
/// - Pagination handling
/// - Delta detection (only emit new items)
/// - Rate limiting
/// - Retry logic with backoff
///
/// # Example
///
/// ```ignore
/// use streamweave::producers::http_poll::{HttpPollProducer, HttpPollProducerConfig};
/// use std::time::Duration;
///
/// let producer = HttpPollProducer::new(
///     HttpPollProducerConfig::default()
///         .with_url("https://api.example.com/data")
///         .with_poll_interval(Duration::from_secs(60))
///         .with_rate_limit(10.0) // 10 requests per second
/// );
/// ```
pub struct HttpPollProducer {
  /// Producer configuration.
  pub config: ProducerConfig<HttpPollResponse>,
  /// HTTP polling-specific configuration.
  pub http_config: HttpPollProducerConfig,
  /// HTTP client.
  pub(crate) client: Option<reqwest::Client>,
  /// Rate limiter state.
  #[allow(unused)]
  pub(crate) last_request_time: Option<std::time::Instant>,
}

impl HttpPollProducer {
  /// Creates a new HTTP polling producer with the given configuration.
  #[must_use]
  pub fn new(http_config: HttpPollProducerConfig) -> Self {
    Self {
      config: ProducerConfig::default(),
      http_config,
      client: None,
      last_request_time: None,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<HttpPollResponse>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the HTTP polling configuration.
  #[must_use]
  pub fn http_config(&self) -> &HttpPollProducerConfig {
    &self.http_config
  }
}

impl Clone for HttpPollProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      http_config: self.http_config.clone(),
      client: None, // Client cannot be cloned, will be re-created
      last_request_time: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;

  #[test]
  fn test_http_poll_producer_config_default() {
    let config = HttpPollProducerConfig::default();
    assert_eq!(config.method, "GET");
    assert_eq!(config.poll_interval, Duration::from_secs(60));
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.retries, 3);
  }

  #[test]
  fn test_http_poll_producer_config_builder() {
    let config = HttpPollProducerConfig::default()
      .with_url("https://api.example.com/data")
      .with_method("POST")
      .with_poll_interval(Duration::from_secs(30))
      .with_max_requests(100)
      .with_rate_limit(5.0)
      .with_body(r#"{"query": "test"}"#);

    assert_eq!(config.url, "https://api.example.com/data");
    assert_eq!(config.method, "POST");
    assert_eq!(config.poll_interval, Duration::from_secs(30));
    assert_eq!(config.max_requests, Some(100));
    assert_eq!(config.rate_limit, Some(5.0));
    assert_eq!(config.body, Some(r#"{"query": "test"}"#.to_string()));
  }

  #[test]
  fn test_delta_detection_config() {
    let mut seen_ids = HashSet::new();
    seen_ids.insert("1".to_string());
    seen_ids.insert("2".to_string());
    let config = DeltaDetectionConfig::with_seen_ids("id", seen_ids);
    assert_eq!(config.id_field, "id");
    assert_eq!(config.seen_ids.len(), 2);
  }

  #[test]
  fn test_pagination_config_query_param() {
    let pagination = PaginationConfig::QueryParameter {
      page_param: "page".to_string(),
      limit_param: Some("limit".to_string()),
      step: 1,
      start: 0,
      max_pages: Some(10),
    };
    match pagination {
      PaginationConfig::QueryParameter {
        page_param,
        limit_param,
        step,
        start,
        max_pages,
      } => {
        assert_eq!(page_param, "page");
        assert_eq!(limit_param, Some("limit".to_string()));
        assert_eq!(step, 1);
        assert_eq!(start, 0);
        assert_eq!(max_pages, Some(10));
      }
      _ => panic!("Expected QueryParameter variant"),
    }
  }

  #[test]
  fn test_http_poll_producer_new() {
    let http_config = HttpPollProducerConfig::default().with_url("https://example.com");
    let producer = HttpPollProducer::new(http_config);
    assert_eq!(producer.http_config().url, "https://example.com");
  }
}
