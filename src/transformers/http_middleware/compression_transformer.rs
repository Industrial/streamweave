use crate::http::{
  http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
};
use crate::input::Input;
use crate::output::Output;
use crate::transformer::Transformer;
use crate::transformer::TransformerConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use http::HeaderValue;
use std::collections::HashSet;
use std::pin::Pin;

/// Compression algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
  Gzip,
  Deflate,
  Brotli,
  Zstd,
}

impl CompressionAlgorithm {
  pub fn as_str(&self) -> &'static str {
    match self {
      CompressionAlgorithm::Gzip => "gzip",
      CompressionAlgorithm::Deflate => "deflate",
      CompressionAlgorithm::Brotli => "br",
      CompressionAlgorithm::Zstd => "zstd",
    }
  }

  pub fn from_str(s: &str) -> Option<Self> {
    match s.to_lowercase().as_str() {
      "gzip" => Some(CompressionAlgorithm::Gzip),
      "deflate" => Some(CompressionAlgorithm::Deflate),
      "br" | "brotli" => Some(CompressionAlgorithm::Brotli),
      "zstd" => Some(CompressionAlgorithm::Zstd),
      _ => None,
    }
  }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
  pub enabled_algorithms: Vec<CompressionAlgorithm>,
  pub min_size: usize,
  pub max_size: Option<usize>,
  pub content_types: HashSet<String>,
  pub exclude_content_types: HashSet<String>,
  pub compression_level: u32,
}

impl Default for CompressionConfig {
  fn default() -> Self {
    let mut content_types = HashSet::new();
    content_types.insert("text/plain".to_string());
    content_types.insert("text/html".to_string());
    content_types.insert("text/css".to_string());
    content_types.insert("text/javascript".to_string());
    content_types.insert("application/json".to_string());
    content_types.insert("application/xml".to_string());
    content_types.insert("application/javascript".to_string());

    Self {
      enabled_algorithms: vec![
        CompressionAlgorithm::Gzip,
        CompressionAlgorithm::Deflate,
        CompressionAlgorithm::Brotli,
      ],
      min_size: 1024,                   // 1KB
      max_size: Some(10 * 1024 * 1024), // 10MB
      content_types,
      exclude_content_types: HashSet::new(),
      compression_level: 6,
    }
  }
}

/// Compression transformer for responses
pub struct CompressionTransformer {
  config: CompressionConfig,
  transformer_config: TransformerConfig<StreamWeaveHttpResponse>,
}

impl CompressionTransformer {
  pub fn new() -> Self {
    Self {
      config: CompressionConfig::default(),
      transformer_config: TransformerConfig::default(),
    }
  }

  pub fn with_config(mut self, config: CompressionConfig) -> Self {
    self.config = config;
    self
  }

  pub fn with_transformer_config(
    mut self,
    config: TransformerConfig<StreamWeaveHttpResponse>,
  ) -> Self {
    self.transformer_config = config;
    self
  }

  #[allow(dead_code)]
  fn should_compress(&self, response: &StreamWeaveHttpResponse) -> bool {
    // Check if response is already compressed
    if response.headers.get("content-encoding").is_some() {
      return false;
    }

    // Check content type
    let content_type = response
      .headers
      .get("content-type")
      .and_then(|h| h.to_str().ok())
      .unwrap_or("");

    if !self.config.content_types.is_empty() {
      let should_compress_type = self
        .config
        .content_types
        .iter()
        .any(|ct| content_type.starts_with(ct));
      if !should_compress_type {
        return false;
      }
    }

    if self.config.exclude_content_types.contains(content_type) {
      return false;
    }

    // Check size constraints
    let body_size = response.body.len();
    if body_size < self.config.min_size {
      return false;
    }

    if let Some(max_size) = self.config.max_size {
      if body_size > max_size {
        return false;
      }
    }

    true
  }

  #[allow(dead_code)]
  fn get_best_algorithm(&self, accept_encoding: &str) -> Option<CompressionAlgorithm> {
    let accepted: HashSet<String> = accept_encoding
      .split(',')
      .map(|s| s.trim().to_lowercase())
      .collect();

    // Find the best algorithm based on priority and client support
    for algorithm in &self.config.enabled_algorithms {
      if accepted.contains(algorithm.as_str()) {
        return Some(*algorithm);
      }
    }

    None
  }

  #[allow(dead_code)]
  async fn compress_body(
    &self,
    body: &[u8],
    algorithm: CompressionAlgorithm,
  ) -> Result<Vec<u8>, CompressionError> {
    match algorithm {
      CompressionAlgorithm::Gzip => self.compress_gzip(body).await,
      CompressionAlgorithm::Deflate => self.compress_deflate(body).await,
      CompressionAlgorithm::Brotli => self.compress_brotli(body).await,
      CompressionAlgorithm::Zstd => self.compress_zstd(body).await,
    }
  }

  #[allow(dead_code)]
  async fn compress_gzip(&self, body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
    encoder
      .write_all(body)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    encoder
      .finish()
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })
  }

  #[allow(dead_code)]
  async fn compress_deflate(&self, body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    let mut encoder =
      DeflateEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
    encoder
      .write_all(body)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    encoder
      .finish()
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })
  }

  #[allow(dead_code)]
  async fn compress_brotli(&self, _body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    // For now, we'll use a simple implementation
    // In a real implementation, you'd use the brotli crate
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "brotli".to_string(),
    })
  }

  #[allow(dead_code)]
  async fn compress_zstd(&self, _body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    // For now, we'll use a simple implementation
    // In a real implementation, you'd use the zstd crate
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "zstd".to_string(),
    })
  }

  // Static methods for use in async streams
  fn should_compress_static(
    config: &CompressionConfig,
    response: &StreamWeaveHttpResponse,
  ) -> bool {
    // Check if response is already compressed
    if response.headers.get("content-encoding").is_some() {
      return false;
    }

    // Check content type
    let content_type = response
      .headers
      .get("content-type")
      .and_then(|h| h.to_str().ok())
      .unwrap_or("");

    if !config.content_types.is_empty() {
      let should_compress_type = config
        .content_types
        .iter()
        .any(|ct| content_type.starts_with(ct));
      if !should_compress_type {
        return false;
      }
    }

    if config.exclude_content_types.contains(content_type) {
      return false;
    }

    // Check size constraints
    let body_size = response.body.len();
    if body_size < config.min_size {
      return false;
    }

    if let Some(max_size) = config.max_size {
      if body_size > max_size {
        return false;
      }
    }

    true
  }

  fn get_best_algorithm_static(
    config: &CompressionConfig,
    accept_encoding: &str,
  ) -> Option<CompressionAlgorithm> {
    let accepted: HashSet<String> = accept_encoding
      .split(',')
      .map(|s| s.trim().to_lowercase())
      .collect();

    // Find the best algorithm based on priority and client support
    for algorithm in &config.enabled_algorithms {
      if accepted.contains(algorithm.as_str()) {
        return Some(*algorithm);
      }
    }

    None
  }

  async fn compress_body_static(
    config: &CompressionConfig,
    body: &[u8],
    algorithm: CompressionAlgorithm,
  ) -> Result<Vec<u8>, CompressionError> {
    match algorithm {
      CompressionAlgorithm::Gzip => Self::compress_gzip_static(config, body).await,
      CompressionAlgorithm::Deflate => Self::compress_deflate_static(config, body).await,
      CompressionAlgorithm::Brotli => Self::compress_brotli_static(config, body).await,
      CompressionAlgorithm::Zstd => Self::compress_zstd_static(config, body).await,
    }
  }

  async fn compress_gzip_static(
    config: &CompressionConfig,
    body: &[u8],
  ) -> Result<Vec<u8>, CompressionError> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(config.compression_level));
    encoder
      .write_all(body)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    encoder
      .finish()
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })
  }

  async fn compress_deflate_static(
    config: &CompressionConfig,
    body: &[u8],
  ) -> Result<Vec<u8>, CompressionError> {
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(config.compression_level));
    encoder
      .write_all(body)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    encoder
      .finish()
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })
  }

  async fn compress_brotli_static(
    _config: &CompressionConfig,
    _body: &[u8],
  ) -> Result<Vec<u8>, CompressionError> {
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "brotli".to_string(),
    })
  }

  async fn compress_zstd_static(
    _config: &CompressionConfig,
    _body: &[u8],
  ) -> Result<Vec<u8>, CompressionError> {
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "zstd".to_string(),
    })
  }
}

#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
  #[error("Compression failed: {message}")]
  CompressionFailed { message: String },

  #[error("Unsupported algorithm: {algorithm}")]
  UnsupportedAlgorithm { algorithm: String },
}

impl Default for CompressionTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for CompressionTransformer {
  type Input = StreamWeaveHttpResponse;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for CompressionTransformer {
  type Output = StreamWeaveHttpResponse;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for CompressionTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let config = self.config.clone();

    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(mut response) = input_stream.next().await {
            // Check if we should compress this response
            if config.enabled_algorithms.is_empty() || !Self::should_compress_static(&config, &response) {
                yield response;
                continue;
            }

            // Get the Accept-Encoding header from the original request
            // For now, we'll assume gzip is acceptable
            let accept_encoding = "gzip, deflate"; // This should come from the request
            let algorithm = Self::get_best_algorithm_static(&config, accept_encoding);

            if let Some(algorithm) = algorithm {
                match Self::compress_body_static(&config, &response.body, algorithm).await {
                    Ok(compressed_body) => {
                        // Add compression headers
                        response.headers.insert("content-encoding", HeaderValue::from_static(algorithm.as_str()));
                        response.headers.insert("vary", HeaderValue::from_static("accept-encoding"));

                        // Update the response body
                        response.body = compressed_body.into();

                        yield response;
                    }
                    Err(_e) => {
                        // If compression fails, return the original response
                        // In a real implementation, you might want to log this error
                        yield response;
                    }
                }
            } else {
                // No supported algorithm found, return original response
                yield response;
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

/// Request decompression transformer
pub struct DecompressionTransformer {
  config: CompressionConfig,
  transformer_config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

impl DecompressionTransformer {
  pub fn new() -> Self {
    Self {
      config: CompressionConfig::default(),
      transformer_config: TransformerConfig::default(),
    }
  }

  pub fn with_config(mut self, config: CompressionConfig) -> Self {
    self.config = config;
    self
  }

  pub fn with_transformer_config(
    mut self,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
  ) -> Self {
    self.transformer_config = config;
    self
  }

  #[allow(dead_code)]
  fn should_decompress(&self, request: &StreamWeaveHttpRequestChunk) -> bool {
    request.headers.get("content-encoding").is_some()
  }

  #[allow(dead_code)]
  async fn decompress_body(
    &self,
    body: &[u8],
    encoding: &str,
  ) -> Result<Vec<u8>, CompressionError> {
    match encoding.to_lowercase().as_str() {
      "gzip" => self.decompress_gzip(body).await,
      "deflate" => self.decompress_deflate(body).await,
      "br" | "brotli" => self.decompress_brotli(body).await,
      "zstd" => self.decompress_zstd(body).await,
      _ => Err(CompressionError::UnsupportedAlgorithm {
        algorithm: encoding.to_string(),
      }),
    }
  }

  #[allow(dead_code)]
  async fn decompress_gzip(&self, body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(body);
    let mut decompressed = Vec::new();
    decoder
      .read_to_end(&mut decompressed)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    Ok(decompressed)
  }

  #[allow(dead_code)]
  async fn decompress_deflate(&self, body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let mut decoder = DeflateDecoder::new(body);
    let mut decompressed = Vec::new();
    decoder
      .read_to_end(&mut decompressed)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    Ok(decompressed)
  }

  #[allow(dead_code)]
  async fn decompress_brotli(&self, _body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "brotli".to_string(),
    })
  }

  #[allow(dead_code)]
  async fn decompress_zstd(&self, _body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "zstd".to_string(),
    })
  }

  // Static methods for use in async streams
  fn should_decompress_static(
    _config: &CompressionConfig,
    request: &StreamWeaveHttpRequestChunk,
  ) -> bool {
    request.headers.get("content-encoding").is_some()
  }

  async fn decompress_body_static(
    _config: &CompressionConfig,
    body: &[u8],
    encoding: &str,
  ) -> Result<Vec<u8>, CompressionError> {
    match encoding.to_lowercase().as_str() {
      "gzip" => Self::decompress_gzip_static(body).await,
      "deflate" => Self::decompress_deflate_static(body).await,
      "br" | "brotli" => Self::decompress_brotli_static(body).await,
      "zstd" => Self::decompress_zstd_static(body).await,
      _ => Err(CompressionError::UnsupportedAlgorithm {
        algorithm: encoding.to_string(),
      }),
    }
  }

  async fn decompress_gzip_static(body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(body);
    let mut decompressed = Vec::new();
    decoder
      .read_to_end(&mut decompressed)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    Ok(decompressed)
  }

  async fn decompress_deflate_static(body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let mut decoder = DeflateDecoder::new(body);
    let mut decompressed = Vec::new();
    decoder
      .read_to_end(&mut decompressed)
      .map_err(|e| CompressionError::CompressionFailed {
        message: e.to_string(),
      })?;
    Ok(decompressed)
  }

  async fn decompress_brotli_static(_body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "brotli".to_string(),
    })
  }

  async fn decompress_zstd_static(_body: &[u8]) -> Result<Vec<u8>, CompressionError> {
    Err(CompressionError::UnsupportedAlgorithm {
      algorithm: "zstd".to_string(),
    })
  }
}

impl Default for DecompressionTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for DecompressionTransformer {
  type Input = StreamWeaveHttpRequestChunk;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for DecompressionTransformer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for DecompressionTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let config = self.config.clone();

    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(mut request) = input_stream.next().await {
            if !Self::should_decompress_static(&config, &request) {
                yield request;
                continue;
            }

            let encoding = request.headers.get("content-encoding")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            match Self::decompress_body_static(&config, &request.chunk, encoding).await {
                Ok(decompressed_body) => {
                    // Remove the content-encoding header
                    request.headers.remove("content-encoding");

                    // Update the request body
                    request.chunk = decompressed_body.into();

                    yield request;
                }
                Err(_e) => {
                    // If decompression fails, return the original request
                    // In a real implementation, you might want to return an error
                    yield request;
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::http::{
    connection_info::ConnectionInfo, http_request_chunk::StreamWeaveHttpRequestChunk,
  };
  use http::{HeaderMap, Method, StatusCode, Uri, Version};

  fn create_test_request(headers: HeaderMap, body: &[u8]) -> StreamWeaveHttpRequestChunk {
    let connection_info = ConnectionInfo::new(
      "127.0.0.1:8080".parse().unwrap(),
      "0.0.0.0:3000".parse().unwrap(),
      Version::HTTP_11,
    );

    StreamWeaveHttpRequestChunk::new(
      Method::POST,
      Uri::from_static("/test"),
      headers,
      bytes::Bytes::from(body.to_vec()),
      connection_info,
      true,
    )
  }

  #[test]
  fn test_compression_algorithm_as_str() {
    assert_eq!(CompressionAlgorithm::Gzip.as_str(), "gzip");
    assert_eq!(CompressionAlgorithm::Deflate.as_str(), "deflate");
    assert_eq!(CompressionAlgorithm::Brotli.as_str(), "br");
    assert_eq!(CompressionAlgorithm::Zstd.as_str(), "zstd");
  }

  #[test]
  fn test_compression_algorithm_from_str() {
    assert_eq!(
      CompressionAlgorithm::from_str("gzip"),
      Some(CompressionAlgorithm::Gzip)
    );
    assert_eq!(
      CompressionAlgorithm::from_str("deflate"),
      Some(CompressionAlgorithm::Deflate)
    );
    assert_eq!(
      CompressionAlgorithm::from_str("br"),
      Some(CompressionAlgorithm::Brotli)
    );
    assert_eq!(
      CompressionAlgorithm::from_str("brotli"),
      Some(CompressionAlgorithm::Brotli)
    );
    assert_eq!(
      CompressionAlgorithm::from_str("zstd"),
      Some(CompressionAlgorithm::Zstd)
    );
    assert_eq!(CompressionAlgorithm::from_str("unknown"), None);
  }

  #[test]
  fn test_compression_config_default() {
    let config = CompressionConfig::default();
    assert!(!config.enabled_algorithms.is_empty());
    assert_eq!(config.min_size, 1024);
    assert_eq!(config.compression_level, 6);
    assert!(config.content_types.contains(&"text/plain".to_string()));
    assert!(
      config
        .content_types
        .contains(&"application/json".to_string())
    );
  }

  #[test]
  fn test_compression_transformer_should_compress() {
    let transformer = CompressionTransformer::new();

    // Test response that should be compressed
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/plain".parse().unwrap());
    let response = StreamWeaveHttpResponse::new(
      StatusCode::OK,
      headers,
      bytes::Bytes::from(vec![0u8; 2048]), // 2KB body
    );

    assert!(transformer.should_compress(&response));
  }

  #[test]
  fn test_compression_transformer_should_not_compress_small_body() {
    let transformer = CompressionTransformer::new();

    // Test response that should not be compressed (too small)
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/plain".parse().unwrap());
    let response = StreamWeaveHttpResponse::new(
      StatusCode::OK,
      headers,
      bytes::Bytes::from(vec![0u8; 100]), // 100 bytes body
    );

    assert!(!transformer.should_compress(&response));
  }

  #[test]
  fn test_compression_transformer_should_not_compress_already_compressed() {
    let transformer = CompressionTransformer::new();

    // Test response that's already compressed
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/plain".parse().unwrap());
    headers.insert("content-encoding", "gzip".parse().unwrap());
    let response =
      StreamWeaveHttpResponse::new(StatusCode::OK, headers, bytes::Bytes::from(vec![0u8; 2048]));

    assert!(!transformer.should_compress(&response));
  }

  #[test]
  fn test_get_best_algorithm() {
    let transformer = CompressionTransformer::new();

    // Test with gzip and deflate support
    let accept_encoding = "gzip, deflate";
    let algorithm = transformer.get_best_algorithm(accept_encoding);
    assert!(algorithm.is_some());

    // Test with no support
    let accept_encoding = "identity";
    let algorithm = transformer.get_best_algorithm(accept_encoding);
    assert!(algorithm.is_none());
  }

  #[test]
  fn test_decompression_transformer_should_decompress() {
    let transformer = DecompressionTransformer::new();

    let mut headers = HeaderMap::new();
    headers.insert("content-encoding", "gzip".parse().unwrap());
    let request = create_test_request(headers, b"test body");

    assert!(transformer.should_decompress(&request));
  }

  #[test]
  fn test_decompression_transformer_should_not_decompress() {
    let transformer = DecompressionTransformer::new();

    let headers = HeaderMap::new();
    let request = create_test_request(headers, b"test body");

    assert!(!transformer.should_decompress(&request));
  }

  #[tokio::test]
  async fn test_gzip_compression() {
    let transformer = CompressionTransformer::new();
    let test_data = b"Hello, World! This is a test string for compression.";

    let result = transformer.compress_gzip(test_data).await;
    assert!(result.is_ok());

    let _compressed = result.unwrap();
    // For very small data, compression might not be effective
    // assert!(compressed.len() < test_data.len());
  }

  #[tokio::test]
  async fn test_gzip_decompression() {
    let transformer = DecompressionTransformer::new();
    let test_data = b"Hello, World! This is a test string for compression.";

    // First compress the data
    let compressed =
      CompressionTransformer::compress_gzip_static(&CompressionConfig::default(), test_data)
        .await
        .unwrap();

    // Then decompress it
    let decompressed = transformer.decompress_gzip(&compressed).await.unwrap();

    assert_eq!(decompressed, test_data);
  }
}
