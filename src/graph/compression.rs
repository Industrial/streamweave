//! # Compression Module
//!
//! This module provides compression and decompression functionality for
//! distributed execution mode. It supports both Gzip and Zstd compression
//! algorithms with configurable compression levels.
//!
//! This module uses `bytes::Bytes` for zero-copy operations, allowing
//! compressed data to be shared efficiently without copying.
//!
//! ## `Message<T>` Support
//!
//! Compression works on serialized `Message<T>` data. In distributed execution mode,
//! `Message<T>` instances are serialized to `Bytes` and then optionally compressed
//! before transmission. During decompression, the serialized `Message<T>` is restored
//! with all IDs and metadata preserved.
//!
//! ## Usage
//!
//! ```rust
//! use crate::graph::compression::{Compression, GzipCompression, ZstdCompression};
//! use bytes::Bytes;
//!
//! // Create a gzip compressor with level 6
//! let gzip = GzipCompression::new(6);
//! let data = b"hello world";
//! let compressed = gzip.compress(data)?;
//! let decompressed = gzip.decompress(&compressed)?;
//! assert_eq!(data, decompressed.as_ref());
//!
//! // Create a zstd compressor with level 3
//! let zstd = ZstdCompression::new(3);
//! let compressed = zstd.compress(data)?;
//! let decompressed = zstd.decompress(&compressed)?;
//! assert_eq!(data, decompressed.as_ref());
//! ```
//!
//! ## Usage with `Message<T>`
//!
//! ```rust
//! use crate::graph::compression::GzipCompression;
//! use crate::graph::serialization::serialize;
//! use crate::message::wrap_message;
//!
//! // Serialize Message<T> to Bytes
//! let msg = wrap_message(42i32);
//! let serialized = serialize(&msg)?;
//!
//! // Compress serialized Message<T>
//! let gzip = GzipCompression::new(6);
//! let compressed = gzip.compress(serialized.as_ref())?;
//!
//! // Decompress and deserialize Message<T> (IDs and metadata preserved)
//! let decompressed = gzip.decompress(compressed.as_ref())?;
//! // let deserialized: Message<i32> = deserialize(decompressed)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use super::execution::CompressionAlgorithm;
use bytes::Bytes;
use flate2::Compression as Flate2Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::{Read, Write};
use thiserror::Error;
use zstd;

/// Error types for compression operations.
#[derive(Error, Debug)]
pub enum CompressionError {
  #[error("Compression failed: {0}")]
  CompressionFailed(String),
  #[error("Decompression failed: {0}")]
  DecompressionFailed(String),
  #[error("Invalid compression level: {0}")]
  InvalidLevel(String),
  #[error("Corrupted compressed data")]
  CorruptedData,
}

/// Trait for compression algorithms.
///
/// This trait abstracts over different compression algorithms, allowing
/// the execution engine to use any compression algorithm interchangeably.
///
/// Uses `Bytes` for zero-copy operations, enabling efficient sharing of
/// compressed data without memory copies.
pub trait Compression: Send + Sync {
  /// Compress the given data.
  ///
  /// # Arguments
  ///
  /// * `data` - The data to compress
  ///
  /// # Returns
  ///
  /// Compressed data as `Bytes` for zero-copy sharing, or an error if compression fails.
  fn compress(&self, data: &[u8]) -> Result<Bytes, CompressionError>;

  /// Decompress the given compressed data.
  ///
  /// # Arguments
  ///
  /// * `data` - The compressed data to decompress
  ///
  /// # Returns
  ///
  /// Decompressed data as `Bytes` for zero-copy sharing, or an error if decompression fails.
  fn decompress(&self, data: &[u8]) -> Result<Bytes, CompressionError>;

  /// Get the compression level.
  ///
  /// # Returns
  ///
  /// The compression level (1-9 for gzip, 1-22 for zstd).
  fn compression_level(&self) -> u32;

  /// Get the compression algorithm type.
  ///
  /// # Returns
  ///
  /// The `CompressionAlgorithm` variant for this compressor.
  fn algorithm(&self) -> CompressionAlgorithm;
}

/// Gzip compression implementation using flate2.
///
/// Supports compression levels 1-9, where:
/// - 1: Fastest compression, largest output
/// - 6: Default, good balance
/// - 9: Slowest compression, smallest output
pub struct GzipCompression {
  level: u32,
}

impl GzipCompression {
  /// Create a new Gzip compressor with the specified level.
  ///
  /// # Arguments
  ///
  /// * `level` - Compression level (1-9). Defaults to 6 if out of range.
  ///
  /// # Returns
  ///
  /// A new `GzipCompression` instance.
  pub fn new(level: u32) -> Self {
    Self {
      level: level.clamp(1, 9),
    }
  }

  /// Create a new Gzip compressor with default level (6).
  #[allow(clippy::should_implement_trait)]
  pub fn default() -> Self {
    Self::new(6)
  }
}

impl Compression for GzipCompression {
  fn compress(&self, data: &[u8]) -> Result<Bytes, CompressionError> {
    let mut encoder = GzEncoder::new(Vec::new(), Flate2Compression::new(self.level));
    encoder
      .write_all(data)
      .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
    let compressed = encoder
      .finish()
      .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
    Ok(Bytes::from(compressed))
  }

  fn decompress(&self, data: &[u8]) -> Result<Bytes, CompressionError> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|e| {
      // Check for various error kinds that indicate corrupted data
      match e.kind() {
        std::io::ErrorKind::InvalidData | std::io::ErrorKind::InvalidInput => {
          CompressionError::CorruptedData
        }
        _ => {
          // Also check error message for corruption indicators
          let msg = e.to_string().to_lowercase();
          if msg.contains("corrupt") || msg.contains("invalid") || msg.contains("stream") {
            CompressionError::CorruptedData
          } else {
            CompressionError::DecompressionFailed(e.to_string())
          }
        }
      }
    })?;
    Ok(Bytes::from(decompressed))
  }

  fn compression_level(&self) -> u32 {
    self.level
  }

  fn algorithm(&self) -> CompressionAlgorithm {
    CompressionAlgorithm::Gzip { level: self.level }
  }
}

/// Zstd compression implementation using zstd-rs.
///
/// Supports compression levels 1-22, where:
/// - 1: Fastest compression, largest output
/// - 3: Default, good balance
/// - 22: Slowest compression, smallest output
pub struct ZstdCompression {
  level: i32,
}

impl ZstdCompression {
  /// Create a new Zstd compressor with the specified level.
  ///
  /// # Arguments
  ///
  /// * `level` - Compression level (1-22). Defaults to 3 if out of range.
  ///
  /// # Returns
  ///
  /// A new `ZstdCompression` instance.
  pub fn new(level: u32) -> Self {
    Self {
      level: level.clamp(1, 22) as i32,
    }
  }

  /// Create a new Zstd compressor with default level (3).
  #[allow(clippy::should_implement_trait)]
  pub fn default() -> Self {
    Self::new(3)
  }
}

impl Compression for ZstdCompression {
  fn compress(&self, data: &[u8]) -> Result<Bytes, CompressionError> {
    zstd::encode_all(data, self.level)
      .map_err(|e| CompressionError::CompressionFailed(format!("Zstd compression failed: {}", e)))
      .map(Bytes::from)
  }

  fn decompress(&self, data: &[u8]) -> Result<Bytes, CompressionError> {
    zstd::decode_all(data)
      .map_err(|e| {
        if e.to_string().contains("corrupt") || e.to_string().contains("invalid") {
          CompressionError::CorruptedData
        } else {
          CompressionError::DecompressionFailed(format!("Zstd decompression failed: {}", e))
        }
      })
      .map(Bytes::from)
  }

  fn compression_level(&self) -> u32 {
    self.level as u32
  }

  fn algorithm(&self) -> CompressionAlgorithm {
    CompressionAlgorithm::Zstd {
      level: self.level as u32,
    }
  }
}

/// Create a compression implementation from a `CompressionAlgorithm`.
///
/// # Arguments
///
/// * `algorithm` - The compression algorithm with level
///
/// # Returns
///
/// A boxed `Compression` trait object.
pub fn create_compression(algorithm: CompressionAlgorithm) -> Box<dyn Compression> {
  match algorithm {
    CompressionAlgorithm::Gzip { level } => Box::new(GzipCompression::new(level)),
    CompressionAlgorithm::Zstd { level } => Box::new(ZstdCompression::new(level)),
  }
}
