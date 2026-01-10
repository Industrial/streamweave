//! Parquet read node for StreamWeave graphs
//!
//! Reads Parquet files from file paths. Takes file paths as input and outputs
//! Arrow RecordBatches, enabling processing of multiple Parquet files in a pipeline.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ParquetReadTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that reads Parquet files from input paths.
///
/// This node wraps `ParquetReadTransformer` for use in graphs. It takes file paths
/// (String) as input and outputs Arrow RecordBatches, enabling processing
/// of multiple Parquet files in a pipeline.
///
/// Parquet is a columnar storage format that's efficient for analytics workloads.
/// This node reads data in batches and yields Arrow RecordBatch objects.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{ParquetRead, TransformerNode};
///
/// let parquet_read = ParquetRead::new();
/// let node = TransformerNode::from_transformer(
///     "parquet_read".to_string(),
///     parquet_read,
/// );
/// ```
pub struct ParquetRead {
  /// The underlying Parquet read transformer
  transformer: ParquetReadTransformer,
}

impl ParquetRead {
  /// Creates a new `ParquetRead` node with default configuration.
  pub fn new() -> Self {
    Self {
      transformer: ParquetReadTransformer::new(),
    }
  }

  /// Sets the batch size for reading.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Number of rows per batch.
  pub fn with_batch_size(mut self, batch_size: usize) -> Self {
    self.transformer = self.transformer.with_batch_size(batch_size);
    self
  }

  /// Sets the column projection.
  ///
  /// # Arguments
  ///
  /// * `projection` - Column indices to read (None means read all columns).
  pub fn with_projection(mut self, projection: Vec<usize>) -> Self {
    self.transformer = self.transformer.with_projection(projection);
    self
  }

  /// Sets the row groups to read.
  ///
  /// # Arguments
  ///
  /// * `row_groups` - Row group indices to read (None means read all).
  pub fn with_row_groups(mut self, row_groups: Vec<usize>) -> Self {
    self.transformer = self.transformer.with_row_groups(row_groups);
    self
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Default for ParquetRead {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ParquetRead {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ParquetRead {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for ParquetRead {
  type Output = RecordBatch;
  type OutputStream = Pin<Box<dyn Stream<Item = RecordBatch> + Send>>;
}

#[async_trait]
impl Transformer for ParquetRead {
  type InputPorts = (String,);
  type OutputPorts = (RecordBatch,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
