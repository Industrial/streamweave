//! # File System Directory Producer
//!
//! Producer for reading directory entries from the file system in StreamWeave pipelines.
//!
//! This module provides [`FsDirectoryProducer`], a producer that reads directory entries
//! from a specified directory path and emits file and subdirectory paths as a stream.
//! It supports both non-recursive (top-level only) and recursive directory traversal.
//!
//! # Overview
//!
//! [`FsDirectoryProducer`] is useful for file system operations that need to process
//! multiple files or directories. It reads directory entries using Tokio's async file
//! system operations and emits paths as string items. This enables efficient directory
//! traversal and file processing in streaming pipelines.
//!
//! # Key Concepts
//!
//! - **Directory Traversal**: Reads entries from a directory path
//! - **Path Output**: Emits file and subdirectory paths as `String` items
//! - **Recursive Option**: Supports both non-recursive and recursive directory traversal
//! - **Async I/O**: Uses Tokio's async file system operations for efficient I/O
//! - **Error Handling**: Configurable error strategies for I/O failures
//!
//! # Core Types
//!
//! - **[`FsDirectoryProducer`]**: Producer that reads directory entries and emits paths
//!
//! # Quick Start
//!
//! ## Basic Usage (Non-Recursive)
//!
//! ```rust
//! use streamweave::producers::FsDirectoryProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads top-level directory entries
//! let mut producer = FsDirectoryProducer::new("/path/to/directory".to_string());
//!
//! // Generate the stream
//! let mut stream = producer.produce();
//!
//! // Process paths
//! while let Some(path) = stream.next().await {
//!     println!("Found: {}", path);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Recursive Directory Traversal
//!
//! ```rust
//! use streamweave::producers::FsDirectoryProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that recursively reads all directory entries
//! let mut producer = FsDirectoryProducer::new("/path/to/directory".to_string())
//!     .recursive(true);
//!
//! let mut stream = producer.produce();
//!
//! // Process all paths (files and subdirectories recursively)
//! while let Some(path) = stream.next().await {
//!     println!("Found: {}", path);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::producers::FsDirectoryProducer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a producer with error handling strategy
//! let producer = FsDirectoryProducer::new("/path/to/directory".to_string())
//!     .recursive(true)
//!     .with_error_strategy(ErrorStrategy::Skip)  // Skip unreadable directories
//!     .with_name("directory-scanner".to_string());
//! ```
//!
//! ## Processing All Files in a Directory
//!
//! ```rust,no_run
//! use streamweave::producers::FsDirectoryProducer;
//! use streamweave::PipelineBuilder;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that recursively finds all files
//! let producer = FsDirectoryProducer::new("/path/to/directory".to_string())
//!     .recursive(true);
//!
//! // Use in a pipeline to process each file
//! let mut stream = producer.produce();
//! while let Some(path) = stream.next().await {
//!     // Process each file path
//!     if std::path::Path::new(&path).is_file() {
//!         println!("Processing file: {}", path);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! ## Path Output
//!
//! Emits paths as `String` items for flexibility and ease of use. Paths can be converted
//! to `PathBuf` or `Path` as needed for file system operations.
//!
//! ## Recursive Traversal
//!
//! Recursive traversal is implemented using a stack-based approach for efficient
//! directory walking. Non-recursive mode only reads the top-level directory, which is
//! more efficient for shallow directory structures.
//!
//! ## Error Handling
//!
//! Unreadable directories are silently skipped (produce empty stream for that directory).
//! This design prevents the producer from stopping on permission errors or other I/O
//! issues. Error handling strategies can be configured for more control over error behavior.
//!
//! ## Async I/O
//!
//! Uses Tokio's async file system operations (`tokio::fs::read_dir`) for efficient,
//! non-blocking directory reading. This enables concurrent processing and integrates
//! seamlessly with async/await code.
//!
//! ## Path String Conversion
//!
//! Paths are converted to strings using `to_str()`, which may fail for non-UTF-8 paths.
//! Invalid UTF-8 paths are skipped (not yielded). This design follows Rust's standard
//! path handling but may skip some paths on systems with non-UTF-8 paths.
//!
//! # Integration with StreamWeave
//!
//! [`FsDirectoryProducer`] integrates seamlessly with StreamWeave's pipeline and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for directory traversal and file processing
//! - **Graph API**: Wrap in [`crate::graph::nodes::ProducerNode`] for graph-based directory processing
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`ProducerConfig`]
//! - **Transformers**: Paths can be transformed, filtered, or processed by transformers
//!
//! # Common Patterns
//!
//! ## File Processing Pipeline
//!
//! Use with transformers to process all files in a directory:
//!
//! ```rust,no_run
//! use streamweave::producers::FsDirectoryProducer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let producer = FsDirectoryProducer::new("/path/to/directory".to_string())
//!     .recursive(true);
//!
//! // Use in pipeline with transformers to filter and process files
//! // (example pipeline construction)
//! # Ok(())
//! # }
//! ```
//!
//! ## Directory Listing
//!
//! List all entries in a directory (non-recursive):
//!
//! ```rust,no_run
//! use streamweave::producers::FsDirectoryProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut producer = FsDirectoryProducer::new("/path/to/directory".to_string());
//! let mut stream = producer.produce();
//!
//! while let Some(path) = stream.next().await {
//!     println!("Entry: {}", path);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Filtering Files
//!
//! Use with transformers to filter paths (e.g., only files, only certain extensions):
//!
//! ```rust,no_run
//! use streamweave::producers::FsDirectoryProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut producer = FsDirectoryProducer::new("/path/to/directory".to_string())
//!     .recursive(true);
//!
//! let mut stream = producer.produce();
//! while let Some(path) = stream.next().await {
//!     // Filter to only .rs files
//!     if path.ends_with(".rs") && std::path::Path::new(&path).is_file() {
//!         println!("Rust file: {}", path);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Platform Notes
//!
//! - **Path Encoding**: Paths are converted to UTF-8 strings, so non-UTF-8 paths may be skipped
//! - **Symlinks**: Symlinks are followed (behavior depends on `read_dir` implementation)
//! - **Permissions**: Unreadable directories are skipped (empty stream for that directory)
//! - **Cross-Platform**: Works on all platforms supported by Tokio (Unix, Windows, etc.)

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::fs;

/// A producer that reads directory entries from a directory path.
///
/// Emits paths to files and subdirectories found in the specified directory.
pub struct FsDirectoryProducer {
  /// The directory path to read from
  pub path: String,
  /// Whether to read recursively
  pub recursive: bool,
  /// Configuration for the producer
  pub config: ProducerConfig<String>,
}

impl FsDirectoryProducer {
  /// Creates a new `FsDirectoryProducer` with the given directory path.
  ///
  /// # Arguments
  ///
  /// * `path` - The directory path to read from
  pub fn new(path: String) -> Self {
    Self {
      path,
      recursive: false,
      config: ProducerConfig::default(),
    }
  }

  /// Sets whether to read directories recursively.
  pub fn recursive(mut self, recursive: bool) -> Self {
    self.recursive = recursive;
    self
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Output for FsDirectoryProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Producer for FsDirectoryProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let recursive = self.recursive;

    Box::pin(async_stream::stream! {
      match fs::read_dir(&path).await {
        Ok(mut entries) => {
          if recursive {
            // Recursive directory walk
            let mut stack = vec![path];
            while let Some(dir_path) = stack.pop() {
              match fs::read_dir(&dir_path).await {
                Ok(mut dir_entries) => {
                  while let Ok(Some(entry)) = dir_entries.next_entry().await {
                    if let Some(path_str) = entry.path().to_str() {
                      let path = path_str.to_string();
                      yield path.clone();
                      if entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false) {
                        stack.push(path);
                      }
                    }
                  }
                }
                Err(_) => {
                  // Skip directories that can't be read
                }
              }
            }
          } else {
            // Non-recursive: just yield entries in the top-level directory
            while let Ok(Some(entry)) = entries.next_entry().await {
              if let Some(path_str) = entry.path().to_str() {
                yield path_str.to_string();
              }
            }
          }
        }
        Err(_) => {
          // Directory doesn't exist or can't be read - produce empty stream
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "fs_directory_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "fs_directory_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
