use chrono::{DateTime, Utc};
use std::any::Any;
use std::error::Error;

#[derive(Debug)]
pub struct StreamError {
  pub source: Box<dyn Error + Send + Sync>,
  pub context: ErrorContext,
  pub retries: usize,
  pub component: ComponentInfo,
}

#[derive(Debug)]
pub struct ErrorContext {
  pub timestamp: DateTime<Utc>,
  pub item: Option<Box<dyn Any + Send>>,
  pub stage: PipelineStage,
}

#[derive(Debug)]
pub enum PipelineStage {
  Producer,
  Transformer(String),
  Consumer,
}

#[derive(Debug)]
pub struct ComponentInfo {
  pub name: String,
  pub type_name: String,
}

#[derive(Debug, Clone)]
pub enum ErrorAction {
  Stop,
  Skip,
  Retry,
}

pub enum ErrorStrategy {
  Stop,
  Skip,
  Retry(usize),
  Custom(Box<dyn Fn(&StreamError) -> ErrorAction + Send + Sync>),
}

impl Clone for ErrorStrategy {
  fn clone(&self) -> Self {
    match self {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) => ErrorStrategy::Retry(*n),
      ErrorStrategy::Custom(_) => ErrorStrategy::Stop, // Custom handlers can't be cloned
    }
  }
}

impl std::fmt::Debug for ErrorStrategy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "ErrorStrategy::Stop"),
      ErrorStrategy::Skip => write!(f, "ErrorStrategy::Skip"),
      ErrorStrategy::Retry(n) => write!(f, "ErrorStrategy::Retry({})", n),
      ErrorStrategy::Custom(_) => write!(f, "ErrorStrategy::Custom"),
    }
  }
}

impl std::fmt::Display for ErrorStrategy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "stop processing on error"),
      ErrorStrategy::Skip => write!(f, "skip item on error"),
      ErrorStrategy::Retry(n) => write!(f, "retry up to {} times on error", n),
      ErrorStrategy::Custom(_) => write!(f, "use custom error handler"),
    }
  }
}

#[derive(Debug)]
pub struct PipelineError {
  inner: StreamError,
}

impl PipelineError {
  pub fn new<E>(error: E, context: ErrorContext, component: ComponentInfo) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    Self {
      inner: StreamError {
        source: Box::new(error),
        context,
        retries: 0,
        component,
      },
    }
  }

  pub fn from_stream_error(error: StreamError) -> Self {
    Self { inner: error }
  }

  pub fn context(&self) -> &ErrorContext {
    &self.inner.context
  }

  pub fn component(&self) -> &ComponentInfo {
    &self.inner.component
  }
}

impl std::fmt::Display for PipelineError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Pipeline error in {}: {}",
      self.inner.component.name, self.inner.source
    )
  }
}

impl Error for PipelineError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(&*self.inner.source)
  }
}
