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

// Tests moved from src/
#[test]
fn test_gzip_compression() {
  let compressor = GzipCompression::new(6);
  // Use larger, more compressible data (repeated pattern)
  let data = b"hello world, this is a test string for compression. ".repeat(10);

  let compressed = compressor.compress(&data).unwrap();
  assert!(compressed.len() < data.len());

  let decompressed = compressor.decompress(&compressed).unwrap();
  assert_eq!(data, decompressed.as_ref());
}

#[test]
fn test_gzip_round_trip() {
  let compressor = GzipCompression::default();
  let original = b"test data for round trip compression";

  let compressed = compressor.compress(original).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(original, decompressed.as_ref());
}

#[test]
fn test_zstd_compression() {
  let compressor = ZstdCompression::new(3);
  // Use larger, more compressible data (repeated pattern)
  let data = b"hello world, this is a test string for compression. ".repeat(10);

  let compressed = compressor.compress(&data).unwrap();
  assert!(compressed.len() < data.len());

  let decompressed = compressor.decompress(&compressed).unwrap();
  assert_eq!(data, decompressed.as_ref());
}

#[test]
fn test_zstd_round_trip() {
  let compressor = ZstdCompression::default();
  let original = b"test data for round trip compression";

  let compressed = compressor.compress(original).unwrap();
  let decompressed = compressor.decompress(&compressed).unwrap();

  assert_eq!(original, decompressed.as_ref());
}

#[test]
fn test_gzip_different_levels() {
  let data = b"repeated data repeated data repeated data repeated data";

  let level1 = GzipCompression::new(1);
  let level9 = GzipCompression::new(9);

  let compressed1 = level1.compress(data).unwrap();
  let compressed9 = level9.compress(data).unwrap();

  // Level 9 should produce smaller output (better compression)
  assert!(compressed9.len() <= compressed1.len());

  // Both should decompress correctly
  assert_eq!(data, level1.decompress(&compressed1).unwrap().as_ref());
  assert_eq!(data, level9.decompress(&compressed9).unwrap().as_ref());
}

#[test]
fn test_zstd_different_levels() {
  // Use larger, more compressible data to ensure higher levels show benefit
  let data = b"repeated data repeated data repeated data repeated data ".repeat(20);

  let level1 = ZstdCompression::new(1);
  let level22 = ZstdCompression::new(22);

  let compressed1 = level1.compress(&data).unwrap();
  let compressed22 = level22.compress(&data).unwrap();

  // Level 22 should produce smaller output (better compression) for larger data
  assert!(compressed22.len() <= compressed1.len());

  // Both should decompress correctly
  assert_eq!(data, level1.decompress(&compressed1).unwrap().as_ref());
  assert_eq!(data, level22.decompress(&compressed22).unwrap().as_ref());
}

#[test]
fn test_gzip_corrupted_data() {
  let compressor = GzipCompression::default();
  let corrupted = b"this is not valid gzip data";

  let result = compressor.decompress(corrupted);
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    CompressionError::CorruptedData
  ));
}

#[test]
fn test_zstd_corrupted_data() {
  let compressor = ZstdCompression::default();
  let corrupted = b"this is not valid zstd data";

  let result = compressor.decompress(corrupted);
  assert!(result.is_err());
  // Zstd may return different error types for corrupted data
}

#[test]
fn test_create_compression() {
  let gzip = create_compression(CompressionAlgorithm::Gzip { level: 6 });
  match gzip.algorithm() {
    CompressionAlgorithm::Gzip { level } => assert_eq!(level, 6),
    _ => panic!("Expected Gzip"),
  }
  assert_eq!(gzip.compression_level(), 6);

  let zstd = create_compression(CompressionAlgorithm::Zstd { level: 3 });
  match zstd.algorithm() {
    CompressionAlgorithm::Zstd { level } => assert_eq!(level, 3),
    _ => panic!("Expected Zstd"),
  }
  assert_eq!(zstd.compression_level(), 3);
}

#[test]
fn test_compression_level_clamping() {
  // Test gzip level clamping
  let gzip0 = GzipCompression::new(0);
  assert_eq!(gzip0.compression_level(), 1); // Clamped to 1

  let gzip10 = GzipCompression::new(10);
  assert_eq!(gzip10.compression_level(), 9); // Clamped to 9

  // Test zstd level clamping
  let zstd0 = ZstdCompression::new(0);
  assert_eq!(zstd0.compression_level(), 1); // Clamped to 1

  let zstd25 = ZstdCompression::new(25);
  assert_eq!(zstd25.compression_level(), 22); // Clamped to 22
}
