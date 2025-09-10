use crate::http::{
  http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
};
use crate::input::Input;
use crate::output::Output;
use crate::transformer::Transformer;
use crate::transformer::TransformerConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use http::{HeaderMap, StatusCode};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiting strategy
#[derive(Debug, Clone)]
pub enum RateLimitStrategy {
  /// Fixed window: reset every N seconds
  FixedWindow {
    window_seconds: u64,
    max_requests: u32,
  },
  /// Sliding window: rolling window of N seconds
  SlidingWindow {
    window_seconds: u64,
    max_requests: u32,
  },
  /// Token bucket: refill tokens at a rate
  TokenBucket {
    capacity: u32,
    refill_rate: f64,
    refill_period: Duration,
  },
}

/// Rate limit key extractor
pub trait RateLimitKeyExtractor: Send + Sync {
  fn extract_key(&self, request: &StreamWeaveHttpRequestChunk) -> String;
}

/// IP-based key extractor
pub struct IpKeyExtractor;

impl RateLimitKeyExtractor for IpKeyExtractor {
  fn extract_key(&self, request: &StreamWeaveHttpRequestChunk) -> String {
    request.connection_info.remote_addr.to_string()
  }
}

/// User-based key extractor (requires authentication)
pub struct UserKeyExtractor;

impl RateLimitKeyExtractor for UserKeyExtractor {
  fn extract_key(&self, request: &StreamWeaveHttpRequestChunk) -> String {
    // In a real implementation, this would extract user ID from JWT or session
    request
      .headers
      .get("x-user-id")
      .and_then(|h| h.to_str().ok())
      .map(|s| s.to_string())
      .unwrap_or_else(|| request.connection_info.remote_addr.to_string())
  }
}

/// Custom key extractor
pub struct CustomKeyExtractor {
  extractor: Box<dyn Fn(&StreamWeaveHttpRequestChunk) -> String + Send + Sync>,
}

impl CustomKeyExtractor {
  pub fn new<F>(extractor: F) -> Self
  where
    F: Fn(&StreamWeaveHttpRequestChunk) -> String + Send + Sync + 'static,
  {
    Self {
      extractor: Box::new(extractor),
    }
  }
}

impl RateLimitKeyExtractor for CustomKeyExtractor {
  fn extract_key(&self, request: &StreamWeaveHttpRequestChunk) -> String {
    (self.extractor)(request)
  }
}

/// Rate limit entry for tracking requests
#[derive(Debug, Clone)]
struct RateLimitEntry {
  count: u32,
  window_start: Instant,
  last_request: Instant,
}

impl RateLimitEntry {
  fn new() -> Self {
    let now = Instant::now();
    Self {
      count: 0,
      window_start: now,
      last_request: now,
    }
  }
}

/// Token bucket implementation
#[derive(Debug, Clone)]
struct TokenBucket {
  capacity: u32,
  tokens: f64,
  last_refill: Instant,
  refill_rate: f64,
  refill_period: Duration,
}

impl TokenBucket {
  fn new(capacity: u32, refill_rate: f64, refill_period: Duration) -> Self {
    Self {
      capacity,
      tokens: capacity as f64,
      last_refill: Instant::now(),
      refill_rate,
      refill_period,
    }
  }

  fn try_consume(&mut self, tokens: u32) -> bool {
    self.refill();

    if self.tokens >= tokens as f64 {
      self.tokens -= tokens as f64;
      true
    } else {
      false
    }
  }

  fn refill(&mut self) {
    let now = Instant::now();
    let elapsed = now.duration_since(self.last_refill);

    if elapsed >= self.refill_period {
      let tokens_to_add = self.refill_rate * elapsed.as_secs_f64();
      self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
      self.last_refill = now;
    }
  }

  fn tokens_available(&self) -> f64 {
    self.tokens
  }
}

/// Rate limiter implementation
pub struct RateLimiter {
  strategy: RateLimitStrategy,
  entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
  token_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
  key_extractor: Box<dyn RateLimitKeyExtractor>,
}

impl RateLimiter {
  pub fn new(strategy: RateLimitStrategy, key_extractor: Box<dyn RateLimitKeyExtractor>) -> Self {
    Self {
      strategy,
      entries: Arc::new(RwLock::new(HashMap::new())),
      token_buckets: Arc::new(RwLock::new(HashMap::new())),
      key_extractor,
    }
  }

  pub async fn is_allowed(&self, request: &StreamWeaveHttpRequestChunk) -> RateLimitResult {
    let key = self.key_extractor.extract_key(request);
    let now = Instant::now();

    match &self.strategy {
      RateLimitStrategy::FixedWindow {
        window_seconds,
        max_requests,
      } => {
        self
          .check_fixed_window(&key, *window_seconds, *max_requests, now)
          .await
      }
      RateLimitStrategy::SlidingWindow {
        window_seconds,
        max_requests,
      } => {
        self
          .check_sliding_window(&key, *window_seconds, *max_requests, now)
          .await
      }
      RateLimitStrategy::TokenBucket {
        capacity,
        refill_rate,
        refill_period,
      } => {
        self
          .check_token_bucket(&key, *capacity, *refill_rate, *refill_period, now)
          .await
      }
    }
  }

  async fn check_fixed_window(
    &self,
    key: &str,
    window_seconds: u64,
    max_requests: u32,
    now: Instant,
  ) -> RateLimitResult {
    let mut entries = self.entries.write().await;
    let entry = entries
      .entry(key.to_string())
      .or_insert_with(RateLimitEntry::new);

    // Check if we need to reset the window
    if now.duration_since(entry.window_start).as_secs() >= window_seconds {
      entry.count = 0;
      entry.window_start = now;
    }

    if entry.count < max_requests {
      entry.count += 1;
      entry.last_request = now;
      RateLimitResult::Allowed {
        remaining: max_requests - entry.count,
        reset_time: entry.window_start + Duration::from_secs(window_seconds),
      }
    } else {
      RateLimitResult::RateLimited {
        retry_after: entry.window_start + Duration::from_secs(window_seconds) - now,
      }
    }
  }

  async fn check_sliding_window(
    &self,
    key: &str,
    window_seconds: u64,
    max_requests: u32,
    now: Instant,
  ) -> RateLimitResult {
    let mut entries = self.entries.write().await;
    let entry = entries
      .entry(key.to_string())
      .or_insert_with(RateLimitEntry::new);

    // For sliding window, we need to track individual request times
    // This is a simplified implementation - in production you'd use a more efficient data structure
    let _window_start = now - Duration::from_secs(window_seconds);

    // In a real implementation, you'd maintain a list of request times
    // and remove old ones. For now, we'll use a simple counter approach
    if now.duration_since(entry.window_start).as_secs() >= window_seconds {
      entry.count = 0;
      entry.window_start = now;
    }

    if entry.count < max_requests {
      entry.count += 1;
      entry.last_request = now;
      RateLimitResult::Allowed {
        remaining: max_requests - entry.count,
        reset_time: now + Duration::from_secs(window_seconds),
      }
    } else {
      RateLimitResult::RateLimited {
        retry_after: entry.window_start + Duration::from_secs(window_seconds) - now,
      }
    }
  }

  async fn check_token_bucket(
    &self,
    key: &str,
    capacity: u32,
    refill_rate: f64,
    refill_period: Duration,
    now: Instant,
  ) -> RateLimitResult {
    let mut buckets = self.token_buckets.write().await;
    let bucket = buckets
      .entry(key.to_string())
      .or_insert_with(|| TokenBucket::new(capacity, refill_rate, refill_period));

    if bucket.try_consume(1) {
      RateLimitResult::Allowed {
        remaining: bucket.tokens_available() as u32,
        reset_time: now + refill_period,
      }
    } else {
      RateLimitResult::RateLimited {
        retry_after: refill_period,
      }
    }
  }
}

/// Rate limit result
#[derive(Debug, Clone)]
pub enum RateLimitResult {
  Allowed { remaining: u32, reset_time: Instant },
  RateLimited { retry_after: Duration },
}

/// Rate limiting transformer
pub struct RateLimitTransformer {
  rate_limiter: Arc<RateLimiter>,
  transformer_config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

impl RateLimitTransformer {
  pub fn new(strategy: RateLimitStrategy, key_extractor: Box<dyn RateLimitKeyExtractor>) -> Self {
    Self {
      rate_limiter: Arc::new(RateLimiter::new(strategy, key_extractor)),
      transformer_config: TransformerConfig::default(),
    }
  }

  pub fn with_transformer_config(
    mut self,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
  ) -> Self {
    self.transformer_config = config;
    self
  }

  #[allow(dead_code)]
  fn create_rate_limit_response(&self, result: &RateLimitResult) -> StreamWeaveHttpResponse {
    let mut headers = HeaderMap::new();

    match result {
      RateLimitResult::Allowed {
        remaining,
        reset_time,
      } => {
        headers.insert(
          "x-ratelimit-remaining",
          remaining.to_string().parse().unwrap(),
        );
        headers.insert(
          "x-ratelimit-reset",
          reset_time
            .duration_since(Instant::now())
            .as_secs()
            .to_string()
            .parse()
            .unwrap(),
        );

        StreamWeaveHttpResponse::new(StatusCode::OK, headers, bytes::Bytes::new())
      }
      RateLimitResult::RateLimited { retry_after } => {
        headers.insert(
          "retry-after",
          retry_after.as_secs().to_string().parse().unwrap(),
        );
        headers.insert("x-ratelimit-limit", "0".parse().unwrap());
        headers.insert("x-ratelimit-remaining", "0".parse().unwrap());

        StreamWeaveHttpResponse::new(
          StatusCode::TOO_MANY_REQUESTS,
          headers,
          bytes::Bytes::from("Rate limit exceeded"),
        )
      }
    }
  }
}

impl Input for RateLimitTransformer {
  type Input = StreamWeaveHttpRequestChunk;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for RateLimitTransformer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for RateLimitTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let rate_limiter = self.rate_limiter.clone();
    let transformer_name = self
      .transformer_config
      .name()
      .unwrap_or("rate_limit".to_string());

    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(request) = input_stream.next().await {
            println!("⏱️  [{}] Checking rate limit for request: {} {}", transformer_name, request.method, request.path());

            // Check rate limit
            let rate_limit_result = rate_limiter.is_allowed(&request).await;

            match rate_limit_result {
                RateLimitResult::Allowed { remaining, reset_time } => {
                    println!("   ✅ [{}] Rate limit passed - remaining: {}, reset: {:?}",
                        transformer_name, remaining, reset_time);
                    // Rate limit passed, continue processing
                    yield request;
                }
                RateLimitResult::RateLimited { retry_after } => {
                    println!("   🚫 [{}] Rate limit exceeded - retry after: {:?}",
                        transformer_name, retry_after);
                    // Rate limit exceeded, skip this request
                    // In a real implementation, you might want to return an error response
                    // For now, we'll just skip the request
                    continue;
                }
            }
        }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.transformer_config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.transformer_config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.transformer_config
  }
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
  #[error("Rate limit exceeded")]
  RateLimited,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::http::{
    connection_info::ConnectionInfo, http_request_chunk::StreamWeaveHttpRequestChunk,
  };
  use http::{Method, Uri, Version};
  use std::time::Duration;
  use uuid::Uuid;

  fn create_test_request(headers: HeaderMap) -> StreamWeaveHttpRequestChunk {
    let connection_info = ConnectionInfo::new(
      "127.0.0.1:8080".parse().unwrap(),
      "0.0.0.0:3000".parse().unwrap(),
      Version::HTTP_11,
    );

    StreamWeaveHttpRequestChunk::new(
      Method::GET,
      Uri::from_static("/test"),
      headers,
      bytes::Bytes::new(),
      connection_info,
      true,
      Uuid::new_v4(),
      Uuid::new_v4(),
    )
  }

  #[test]
  fn test_ip_key_extractor() {
    let extractor = IpKeyExtractor;
    let request = create_test_request(HeaderMap::new());

    let key = extractor.extract_key(&request);
    assert_eq!(key, "127.0.0.1:8080");
  }

  #[test]
  fn test_user_key_extractor() {
    let extractor = UserKeyExtractor;
    let mut headers = HeaderMap::new();
    headers.insert("x-user-id", "user123".parse().unwrap());
    let request = create_test_request(headers);

    let key = extractor.extract_key(&request);
    assert_eq!(key, "user123");
  }

  #[test]
  fn test_user_key_extractor_fallback() {
    let extractor = UserKeyExtractor;
    let request = create_test_request(HeaderMap::new());

    let key = extractor.extract_key(&request);
    assert_eq!(key, "127.0.0.1:8080"); // Falls back to IP
  }

  #[test]
  fn test_custom_key_extractor() {
    let extractor = CustomKeyExtractor::new(|req| format!("{}-{}", req.method, req.uri.path()));
    let request = create_test_request(HeaderMap::new());

    let key = extractor.extract_key(&request);
    assert_eq!(key, "GET-/test");
  }

  #[test]
  fn test_token_bucket_creation() {
    let bucket = TokenBucket::new(10, 1.0, Duration::from_secs(1));
    assert_eq!(bucket.tokens_available(), 10.0);
  }

  #[test]
  fn test_token_bucket_consume() {
    let mut bucket = TokenBucket::new(10, 1.0, Duration::from_secs(1));

    assert!(bucket.try_consume(5));
    assert_eq!(bucket.tokens_available(), 5.0);

    assert!(bucket.try_consume(5));
    assert_eq!(bucket.tokens_available(), 0.0);

    assert!(!bucket.try_consume(1));
  }

  #[tokio::test]
  async fn test_fixed_window_rate_limiter() {
    let strategy = RateLimitStrategy::FixedWindow {
      window_seconds: 60,
      max_requests: 5,
    };
    let key_extractor = Box::new(IpKeyExtractor);
    let rate_limiter = RateLimiter::new(strategy, key_extractor);

    let request = create_test_request(HeaderMap::new());

    // First 5 requests should be allowed
    for _ in 0..5 {
      let result = rate_limiter.is_allowed(&request).await;
      match result {
        RateLimitResult::Allowed { remaining, .. } => {
          assert!(remaining < 5);
        }
        RateLimitResult::RateLimited { .. } => {
          panic!("Expected allowed result");
        }
      }
    }

    // 6th request should be rate limited
    let result = rate_limiter.is_allowed(&request).await;
    match result {
      RateLimitResult::Allowed { .. } => {
        panic!("Expected rate limited result");
      }
      RateLimitResult::RateLimited { .. } => {
        // Expected
      }
    }
  }

  #[tokio::test]
  async fn test_token_bucket_rate_limiter() {
    let strategy = RateLimitStrategy::TokenBucket {
      capacity: 5,
      refill_rate: 1.0,
      refill_period: Duration::from_secs(1),
    };
    let key_extractor = Box::new(IpKeyExtractor);
    let rate_limiter = RateLimiter::new(strategy, key_extractor);

    let request = create_test_request(HeaderMap::new());

    // First 5 requests should be allowed
    for _ in 0..5 {
      let result = rate_limiter.is_allowed(&request).await;
      match result {
        RateLimitResult::Allowed { .. } => {
          // Expected
        }
        RateLimitResult::RateLimited { .. } => {
          panic!("Expected allowed result");
        }
      }
    }

    // 6th request should be rate limited
    let result = rate_limiter.is_allowed(&request).await;
    match result {
      RateLimitResult::Allowed { .. } => {
        panic!("Expected rate limited result");
      }
      RateLimitResult::RateLimited { .. } => {
        // Expected
      }
    }
  }

  #[test]
  fn test_rate_limit_transformer_creation() {
    let strategy = RateLimitStrategy::FixedWindow {
      window_seconds: 60,
      max_requests: 100,
    };
    let key_extractor = Box::new(IpKeyExtractor);
    let transformer = RateLimitTransformer::new(strategy, key_extractor);

    // Just test that it can be created
    assert!(transformer.rate_limiter.entries.try_read().is_ok());
  }

  #[test]
  fn test_create_rate_limit_response_allowed() {
    let strategy = RateLimitStrategy::FixedWindow {
      window_seconds: 60,
      max_requests: 100,
    };
    let key_extractor = Box::new(IpKeyExtractor);
    let transformer = RateLimitTransformer::new(strategy, key_extractor);

    let result = RateLimitResult::Allowed {
      remaining: 50,
      reset_time: Instant::now() + Duration::from_secs(30),
    };

    let response = transformer.create_rate_limit_response(&result);
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.headers.contains_key("x-ratelimit-remaining"));
  }

  #[test]
  fn test_create_rate_limit_response_rate_limited() {
    let strategy = RateLimitStrategy::FixedWindow {
      window_seconds: 60,
      max_requests: 100,
    };
    let key_extractor = Box::new(IpKeyExtractor);
    let transformer = RateLimitTransformer::new(strategy, key_extractor);

    let result = RateLimitResult::RateLimited {
      retry_after: Duration::from_secs(30),
    };

    let response = transformer.create_rate_limit_response(&result);
    assert_eq!(response.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers.contains_key("retry-after"));
  }
}
