//! Tests for graph compression module

use bytes::Bytes;
use streamweave::graph::compression::{
  Compression, CompressionError, GzipCompression, ZstdCompression,
};

#[test]
fn test_gzip_compression_new() {
  let gzip = GzipCompression::new(6);
  // Should create successfully
  assert!(true);
}

#[test]
fn test_gzip_compression_compress_decompress() {
  let gzip = GzipCompression::new(6);
  let data = b"hello world";

  let compressed = gzip.compress(data).unwrap();
  assert!(!compressed.is_empty());

  let decompressed = gzip.decompress(compressed.as_ref()).unwrap();
  assert_eq!(decompressed.as_ref(), data);
}

#[test]
fn test_zstd_compression_new() {
  let zstd = ZstdCompression::new(3);
  // Should create successfully
  assert!(true);
}

#[test]
fn test_zstd_compression_compress_decompress() {
  let zstd = ZstdCompression::new(3);
  let data = b"hello world";

  let compressed = zstd.compress(data).unwrap();
  assert!(!compressed.is_empty());

  let decompressed = zstd.decompress(compressed.as_ref()).unwrap();
  assert_eq!(decompressed.as_ref(), data);
}

#[test]
fn test_compression_error_display() {
  let err = CompressionError::CompressionFailed("test".to_string());
  assert!(err.to_string().contains("Compression failed"));

  let err = CompressionError::DecompressionFailed("test".to_string());
  assert!(err.to_string().contains("Decompression failed"));

  let err = CompressionError::InvalidLevel("test".to_string());
  assert!(err.to_string().contains("Invalid compression level"));

  let err = CompressionError::CorruptedData;
  assert!(err.to_string().contains("Corrupted compressed data"));
}

#[test]
fn test_compression_error_is_error() {
  let err = CompressionError::CompressionFailed("test".to_string());
  // Should compile - implements Error trait
  let _: &dyn std::error::Error = &err;
  assert!(true);
}
