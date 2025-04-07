use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorAction {
  Stop,
  Skip,
  Retry,
}

pub enum ErrorStrategy<T> {
  Stop,
  Skip,
  Retry(usize),
  Custom(Box<dyn Fn(&StreamError<T>) -> ErrorAction + Send + Sync>),
}

impl<T> Clone for ErrorStrategy<T> {
  fn clone(&self) -> Self {
    match self {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) => ErrorStrategy::Retry(*n),
      ErrorStrategy::Custom(_) => ErrorStrategy::Stop, // Custom handlers can't be cloned
    }
  }
}

impl<T> fmt::Debug for ErrorStrategy<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "ErrorStrategy::Stop"),
      ErrorStrategy::Skip => write!(f, "ErrorStrategy::Skip"),
      ErrorStrategy::Retry(n) => write!(f, "ErrorStrategy::Retry({})", n),
      ErrorStrategy::Custom(_) => write!(f, "ErrorStrategy::Custom"),
    }
  }
}

impl<T> PartialEq for ErrorStrategy<T> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (ErrorStrategy::Stop, ErrorStrategy::Stop) => true,
      (ErrorStrategy::Skip, ErrorStrategy::Skip) => true,
      (ErrorStrategy::Retry(n1), ErrorStrategy::Retry(n2)) => n1 == n2,
      (ErrorStrategy::Custom(_), ErrorStrategy::Custom(_)) => true,
      _ => false,
    }
  }
}

impl<T> ErrorStrategy<T> {
  pub fn new_custom<F>(f: F) -> Self
  where
    F: Fn(&StreamError<T>) -> ErrorAction + Send + Sync + 'static,
  {
    Self::Custom(Box::new(f))
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreamError<T> {
  pub source: Box<dyn Error + Send + Sync>,
  pub context: ErrorContext<T>,
  pub component: ComponentInfo,
  pub retries: usize,
}

impl<T> StreamError<T> {
  pub fn new(
    source: Box<dyn Error + Send + Sync>,
    context: ErrorContext<T>,
    component: ComponentInfo,
  ) -> Self {
    Self {
      source,
      context,
      component,
      retries: 0,
    }
  }
}

impl<T> fmt::Display for StreamError<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Error in {} ({}): {}",
      self.component.name, self.component.type_name, self.source
    )
  }
}

impl<T> Error for StreamError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(self.source.as_ref())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorContext<T> {
  pub timestamp: chrono::DateTime<chrono::Utc>,
  pub item: Option<T>,
  pub stage: PipelineStage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStage {
  Producer,
  Transformer(String),
  Consumer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentInfo {
  pub name: String,
  pub type_name: String,
}

impl ComponentInfo {
  pub fn new(name: String, type_name: String) -> Self {
    Self { name, type_name }
  }
}

#[derive(Debug)]
pub struct PipelineError<T> {
  inner: StreamError<T>,
}

impl<T: std::fmt::Debug> PipelineError<T> {
  pub fn new<E>(error: E, context: ErrorContext<T>, component: ComponentInfo) -> Self
  where
    E: Error + Send + Sync + 'static,
    T: Send + Sync + 'static,
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

  pub fn from_stream_error(error: StreamError<T>) -> Self {
    Self { inner: error }
  }

  pub fn context(&self) -> &ErrorContext<T> {
    &self.inner.context
  }

  pub fn component(&self) -> &ComponentInfo {
    &self.inner.component
  }
}

impl<T: std::fmt::Debug> std::fmt::Display for PipelineError<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Pipeline error in {}: {}",
      self.inner.component.name, self.inner.source
    )
  }
}

impl<T: std::fmt::Debug> Error for PipelineError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(&*self.inner.source)
  }
}

impl<T: std::fmt::Debug + Send + Sync + 'static> Error for StreamError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(self.source.as_ref())
  }
}
