use async_trait::async_trait;
use csv::Trim;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use std::marker::PhantomData;
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tracing::error;

/// Configuration for CSV reading behavior.
#[derive(Debug, Clone)]
pub struct CsvReadConfig {
  /// Whether the CSV has a header row.
  pub has_headers: bool,
  /// The delimiter character (default: comma).
  pub delimiter: u8,
  /// Whether to allow flexible column counts.
  pub flexible: bool,
  /// Whether to trim whitespace from fields.
  pub trim: bool,
  /// The comment character (None means no comments).
  pub comment: Option<u8>,
  /// The quote character (default: double quote).
  pub quote: u8,
  /// Whether double quotes are used for escaping.
  pub double_quote: bool,
}

impl Default for CsvReadConfig {
  fn default() -> Self {
    Self {
      has_headers: true,
      delimiter: b',',
      flexible: false,
      trim: false,
      comment: None,
      quote: b'"',
      double_quote: true,
    }
  }
}

impl CsvReadConfig {
  /// Sets whether the CSV has a header row.
  #[must_use]
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.has_headers = has_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.delimiter = delimiter;
    self
  }

  /// Sets whether to allow flexible column counts.
  #[must_use]
  pub fn with_flexible(mut self, flexible: bool) -> Self {
    self.flexible = flexible;
    self
  }

  /// Sets whether to trim whitespace from fields.
  #[must_use]
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.trim = trim;
    self
  }

  /// Sets the comment character.
  #[must_use]
  pub fn with_comment(mut self, comment: Option<u8>) -> Self {
    self.comment = comment;
    self
  }
}

/// A producer that reads CSV files and deserializes rows.
///
/// # Example
///
/// ```ignore
/// use streamweave_csv_producer::CsvProducer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let producer = CsvProducer::<Record>::new("data.csv");
/// ```
pub struct CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Path to the CSV file.
  pub path: String,
  /// Producer configuration.
  pub config: ProducerConfig<T>,
  /// CSV-specific configuration.
  pub csv_config: CsvReadConfig,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new CSV producer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
      csv_config: CsvReadConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets whether the CSV has a header row.
  #[must_use]
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.csv_config.has_headers = has_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.csv_config.delimiter = delimiter;
    self
  }

  /// Sets whether to allow flexible column counts.
  #[must_use]
  pub fn with_flexible(mut self, flexible: bool) -> Self {
    self.csv_config.flexible = flexible;
    self
  }

  /// Sets whether to trim whitespace from fields.
  #[must_use]
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.csv_config.trim = trim;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &str {
    &self.path
  }
}

impl<T> Clone for CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
      csv_config: self.csv_config.clone(),
      _phantom: PhantomData,
    }
  }
}

// Trait implementations for CsvProducer

impl<T> Output for CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<T> Producer for CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type OutputPorts = (T,);

  /// Produces a stream of deserialized CSV rows.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and an empty stream is returned.
  /// - If a row cannot be deserialized, the error strategy determines the action.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "csv_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let csv_config = self.csv_config.clone();

    // CSV reading is synchronous, so we use blocking_task wrapper
    Box::pin(async_stream::stream! {
      // Use tokio's blocking spawn for synchronous CSV reading
      let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<T, StreamError<T>>>(100);

      let component_name_clone = component_name.clone();
      let error_strategy_clone = error_strategy.clone();

      let handle = tokio::task::spawn_blocking(move || {
        let file = match File::open(&path) {
          Ok(f) => f,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to open CSV file"
            );
            return;
          }
        };

        let reader = BufReader::new(file);

        let mut csv_reader = csv::ReaderBuilder::new()
          .has_headers(csv_config.has_headers)
          .delimiter(csv_config.delimiter)
          .flexible(csv_config.flexible)
          .trim(if csv_config.trim { Trim::All } else { Trim::None })
          .quote(csv_config.quote)
          .double_quote(csv_config.double_quote)
          .from_reader(reader);

        if let Some(comment) = csv_config.comment {
          csv_reader = csv::ReaderBuilder::new()
            .has_headers(csv_config.has_headers)
            .delimiter(csv_config.delimiter)
            .flexible(csv_config.flexible)
            .trim(if csv_config.trim { Trim::All } else { Trim::None })
            .quote(csv_config.quote)
            .double_quote(csv_config.double_quote)
            .comment(Some(comment))
            .from_reader(BufReader::new(File::open(&path).unwrap()));
        }

        for result in csv_reader.deserialize::<T>() {
          match result {
            Ok(record) => {
              if tx.blocking_send(Ok(record)).is_err() {
                break;
              }
            }
            Err(e) => {
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: component_name_clone.clone(),
                  component_type: std::any::type_name::<CsvProducer<T>>().to_string(),
                },
                ComponentInfo {
                  name: component_name_clone.clone(),
                  type_name: std::any::type_name::<CsvProducer<T>>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy_clone, &stream_error) {
                ErrorAction::Stop => {
                  let _ = tx.blocking_send(Err(stream_error));
                  break;
                }
                ErrorAction::Skip => {
                  // Continue to next record
                }
                ErrorAction::Retry => {
                  // Retry not supported for CSV row reading, skip
                }
              }
            }
          }
        }
      });

      while let Some(result) = rx.recv().await {
        match result {
          Ok(record) => yield record,
          Err(stream_error) => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Error reading CSV record"
            );
            break;
          }
        }
      }

      // Wait for the blocking task to finish
      let _ = handle.await;
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "csv_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "csv_producer".to_string()),
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
