//! Comprehensive property-based tests for compression module
//!
//! This module provides 100% coverage tests using proptest for:
//! - GzipCompression compress/decompress roundtrips
//! - ZstdCompression compress/decompress roundtrips
//! - Compression level validation
//! - Edge cases (empty data, large data, corrupted data)
//! - Error conditions

use bytes::Bytes;
use proptest::prelude::*;
use streamweave_graph::compression::{
  Compression, CompressionError, GzipCompression, ZstdCompression,
};

// ============================================================================
// Property-based test strategies
// ============================================================================

/// Strategy for generating valid gzip compression levels (1-9)
fn gzip_level_strategy() -> impl Strategy<Value = u32> {
  1u32..=9
}

/// Strategy for generating valid zstd compression levels (1-22)
fn zstd_level_strategy() -> impl Strategy<Value = u32> {
  1u32..=22
}

/// Strategy for generating test data
fn test_data_strategy() -> impl Strategy<Value = Vec<u8>> {
  prop::collection::vec(any::<u8>(), 0..=10240) // Up to 10KB
}

// ============================================================================
// GzipCompression Tests
// ============================================================================

#[test]
fn test_gzip_compression_new() {
  let compressor = GzipCompression::new(6);
  assert_eq!(compressor.compression_level(), 6);
  assert_eq!(
    compressor.algorithm(),
    streamweave_graph::execution::CompressionAlgorithm::Gzip
  );
}

#[test]
fn test_gzip_compression_default() {
  let compressor = GzipCompression::default();
  assert_eq!(compressor.compression_level(), 6);
}

#[test]
fn test_gzip_compression_roundtrip() {
  let compressor = GzipCompression::new(6);
  let data = b"hello world, this is a test string";

  let compressed = compressor.compress(data).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(decompressed.as_ref(), data);
}

#[test]
fn test_gzip_compression_empty_data() {
  let compressor = GzipCompression::new(6);
  let data = b"";

  let compressed = compressor.compress(data).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(decompressed.as_ref(), data);
}

#[test]
fn test_gzip_compression_large_data() {
  let compressor = GzipCompression::new(6);
  let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

  let compressed = compressor.compress(&data).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(decompressed.as_ref(), data.as_slice());
}

#[test]
fn test_gzip_compression_different_levels() {
  let data = b"test data for compression";

  for level in 1..=9 {
    let compressor = GzipCompression::new(level);
    let compressed = compressor.compress(data).unwrap();
    let decompressed = compressor.decompress(&compressed).unwrap();
    assert_eq!(decompressed.as_ref(), data);
  }
}

#[test]
fn test_gzip_compression_corrupted_data() {
  let compressor = GzipCompression::new(6);
  let corrupted = b"this is not valid gzip data";

  let result = compressor.decompress(corrupted);
  assert!(result.is_err());

  match result.unwrap_err() {
    CompressionError::DecompressionFailed(_) | CompressionError::CorruptedData => {}
    _ => panic!("Expected decompression error"),
  }
}

#[test]
fn proptest_gzip_compression_roundtrip() {
  let mut runner = proptest::test_runner::TestRunner::default();
  runner
    .run(&test_data_strategy(), |data| {
      let compressor = GzipCompression::new(6);
      let compressed = compressor.compress(&data).unwrap();
      let decompressed = compressor.decompress(&compressed).unwrap();
      prop_assert_eq!(decompressed.as_ref(), data.as_slice());
      Ok(())
    })
    .unwrap();
}

#[test]
fn proptest_gzip_compression_all_levels() {
  let mut runner = proptest::test_runner::TestRunner::default();
  runner
    .run(
      &(gzip_level_strategy(), test_data_strategy()),
      |(level, data)| {
        let compressor = GzipCompression::new(level);
        let compressed = compressor.compress(&data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        prop_assert_eq!(decompressed.as_ref(), data.as_slice());
        Ok(())
      },
    )
    .unwrap();
}

// ============================================================================
// ZstdCompression Tests
// ============================================================================

#[test]
fn test_zstd_compression_new() {
  let compressor = ZstdCompression::new(3);
  assert_eq!(compressor.compression_level(), 3);
  assert_eq!(
    compressor.algorithm(),
    streamweave_graph::execution::CompressionAlgorithm::Zstd
  );
}

#[test]
fn test_zstd_compression_default() {
  let compressor = ZstdCompression::default();
  assert_eq!(compressor.compression_level(), 3);
}

#[test]
fn test_zstd_compression_roundtrip() {
  let compressor = ZstdCompression::new(3);
  let data = b"hello world, this is a test string";

  let compressed = compressor.compress(data).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(decompressed.as_ref(), data);
}

#[test]
fn test_zstd_compression_empty_data() {
  let compressor = ZstdCompression::new(3);
  let data = b"";

  let compressed = compressor.compress(data).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(decompressed.as_ref(), data);
}

#[test]
fn test_zstd_compression_large_data() {
  let compressor = ZstdCompression::new(3);
  let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

  let compressed = compressor.compress(&data).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(decompressed.as_ref(), data.as_slice());
}

#[test]
fn test_zstd_compression_different_levels() {
  let data = b"test data for compression";

  // Test a few representative levels
  for level in [1, 3, 6, 10, 15, 22] {
    let compressor = ZstdCompression::new(level);
    let compressed = compressor.compress(data).unwrap();
    let decompressed = compressor.decompress(&compressed).unwrap();
    assert_eq!(decompressed.as_ref(), data);
  }
}

#[test]
fn test_zstd_compression_corrupted_data() {
  let compressor = ZstdCompression::new(3);
  let corrupted = b"this is not valid zstd data";

  let result = compressor.decompress(corrupted);
  assert!(result.is_err());

  match result.unwrap_err() {
    CompressionError::DecompressionFailed(_) | CompressionError::CorruptedData => {}
    _ => panic!("Expected decompression error"),
  }
}

#[test]
fn proptest_zstd_compression_roundtrip() {
  let mut runner = proptest::test_runner::TestRunner::default();
  runner
    .run(&test_data_strategy(), |data| {
      let compressor = ZstdCompression::new(3);
      let compressed = compressor.compress(&data).unwrap();
      let decompressed = compressor.decompress(&compressed).unwrap();
      prop_assert_eq!(decompressed.as_ref(), data.as_slice());
      Ok(())
    })
    .unwrap();
}

#[test]
fn proptest_zstd_compression_all_levels() {
  let mut runner = proptest::test_runner::TestRunner::default();
  runner
    .run(
      &(zstd_level_strategy(), test_data_strategy()),
      |(level, data)| {
        let compressor = ZstdCompression::new(level);
        let compressed = compressor.compress(&data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        prop_assert_eq!(decompressed.as_ref(), data.as_slice());
        Ok(())
      },
    )
    .unwrap();
}

// ============================================================================
// Cross-Algorithm Tests
// ============================================================================

#[test]
fn test_gzip_vs_zstd_compression_ratio() {
  let data = b"hello world, this is a test string that should compress well";

  let gzip = GzipCompression::new(6);
  let zstd = ZstdCompression::new(3);

  let gzip_compressed = gzip.compress(data).unwrap();
  let zstd_compressed = zstd.compress(data).unwrap();

  // Both should compress the data
  assert!(gzip_compressed.len() < data.len());
  assert!(zstd_compressed.len() < data.len());

  // Both should decompress correctly
  let gzip_decompressed = gzip.decompress(&gzip_compressed).unwrap();
  let zstd_decompressed = zstd.decompress(&zstd_compressed).unwrap();

  assert_eq!(gzip_decompressed.as_ref(), data);
  assert_eq!(zstd_decompressed.as_ref(), data);
}

#[test]
fn test_compression_bytes_type() {
  let compressor = GzipCompression::new(6);
  let data = b"test data";

  let compressed = compressor.compress(data).unwrap();
  assert!(compressed.len() > 0);

  // Verify it's Bytes type
  let _bytes: Bytes = compressed;
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_compression_error_display() {
  let error = CompressionError::CompressionFailed("test error".to_string());
  let display_str = format!("{}", error);
  assert!(display_str.contains("test error"));

  let error = CompressionError::DecompressionFailed("test error".to_string());
  let display_str = format!("{}", error);
  assert!(display_str.contains("test error"));

  let error = CompressionError::InvalidLevel("test error".to_string());
  let display_str = format!("{}", error);
  assert!(display_str.contains("test error"));

  let error = CompressionError::CorruptedData;
  let display_str = format!("{}", error);
  assert!(display_str.contains("Corrupted"));
}

#[test]
fn test_compression_error_debug() {
  let error = CompressionError::CompressionFailed("test".to_string());
  let debug_str = format!("{:?}", error);
  assert!(debug_str.contains("CompressionFailed"));
}
