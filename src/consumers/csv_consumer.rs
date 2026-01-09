use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use csv::{Writer, WriterBuilder};
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::fs::File;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use tracing::{error, warn};

/// Configuration for CSV writing behavior.
#[derive(Debug, Clone)]
pub struct CsvWriteConfig {
  /// Whether to write a header row.
  pub write_headers: bool,
  /// The delimiter character (default: comma).
  pub delimiter: u8,
  /// The quote character (default: double quote).
  pub quote: u8,
  /// Whether double quotes are used for escaping.
  pub double_quote: bool,
  /// Whether to flush after each record.
  pub flush_on_write: bool,
}

impl Default for CsvWriteConfig {
  fn default() -> Self {
    Self {
      write_headers: true,
      delimiter: b',',
      quote: b'"',
      double_quote: true,
      flush_on_write: false,
    }
  }
}

impl CsvWriteConfig {
  /// Sets whether to write a header row.
  #[must_use]
  pub fn with_headers(mut self, write_headers: bool) -> Self {
    self.write_headers = write_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.delimiter = delimiter;
    self
  }

  /// Sets whether to flush after each record.
  #[must_use]
  pub fn with_flush_on_write(mut self, flush: bool) -> Self {
    self.flush_on_write = flush;
    self
  }
}

/// A consumer that writes CSV files.
///
/// # Example
///
/// ```ignore
/// use streamweave_csv_consumer::CsvConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let consumer = CsvConsumer::<Record>::new("output.csv");
/// ```
pub struct CsvConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the CSV file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// CSV-specific configuration.
  pub csv_config: CsvWriteConfig,
  /// Writer handle (opened on first write).
  pub writer: Option<Writer<File>>,
  /// Whether we've written the first record (for header handling).
  pub first_record_written: bool,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> CsvConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new CSV consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      csv_config: CsvWriteConfig::default(),
      writer: None,
      first_record_written: false,
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Sets whether to write a header row.
  #[must_use]
  pub fn with_headers(mut self, write_headers: bool) -> Self {
    self.csv_config.write_headers = write_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.csv_config.delimiter = delimiter;
    self
  }

  /// Sets whether to flush after each record.
  #[must_use]
  pub fn with_flush_on_write(mut self, flush: bool) -> Self {
    self.csv_config.flush_on_write = flush;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

// Trait implementations for CsvConsumer

impl<T> Input for CsvConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl<T> Consumer for CsvConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  /// Consumes a stream and writes each item as a CSV row to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and no data is written.
  /// - If an item cannot be serialized or written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "csv_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let csv_config = self.csv_config.clone();

    // Open the file
    let file = match File::create(&path) {
      Ok(f) => f,
      Err(e) => {
        error!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to create CSV file for writing"
        );
        return;
      }
    };

    // Create CSV writer
    let mut writer = WriterBuilder::new()
      .has_headers(csv_config.write_headers)
      .delimiter(csv_config.delimiter)
      .quote(csv_config.quote)
      .double_quote(csv_config.double_quote)
      .from_writer(file);

    let mut input = std::pin::pin!(input);

    while let Some(item) = input.next().await {
      // Serialize the record
      if let Err(e) = writer.serialize(&item) {
        let stream_error = StreamError::new(
          Box::new(e),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: Some(item.clone()),
            component_name: component_name.clone(),
            component_type: std::any::type_name::<Self>().to_string(),
          },
          ComponentInfo {
            name: component_name.clone(),
            type_name: std::any::type_name::<Self>().to_string(),
          },
        );

        match handle_error_strategy(&error_strategy, &stream_error) {
          ErrorAction::Stop => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Stopping due to serialization error"
            );
            break;
          }
          ErrorAction::Skip => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Skipping item due to serialization error"
            );
            continue;
          }
          ErrorAction::Retry => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Retry not supported for CSV serialization errors, skipping"
            );
            continue;
          }
        }
      }

      // Flush if configured
      if csv_config.flush_on_write
        && let Err(e) = writer.flush()
      {
        warn!(
          component = %component_name,
          error = %e,
          "Failed to flush CSV writer"
        );
      }
    }

    // Final flush
    if let Err(e) = writer.flush() {
      error!(
        component = %component_name,
        error = %e,
        "Failed to flush CSV file"
      );
    }

    self.first_record_written = true;
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
fn handle_error_strategy<T>(strategy: &ErrorStrategy<T>, error: &StreamError<T>) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}
