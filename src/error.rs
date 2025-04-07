use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorAction {
  Stop,
  Skip,
  Retry,
}

pub enum ErrorStrategy<T: std::fmt::Debug + Clone> {
  Stop,
  Skip,
  Retry(usize),
  Custom(Box<dyn Fn(&StreamError<T>) -> ErrorAction + Send + Sync>),
}

impl<T: std::fmt::Debug + Clone> Clone for ErrorStrategy<T> {
  fn clone(&self) -> Self {
    match self {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) => ErrorStrategy::Retry(*n),
      ErrorStrategy::Custom(_) => ErrorStrategy::Stop, // Custom handlers can't be cloned
    }
  }
}

impl<T: std::fmt::Debug + Clone> fmt::Debug for ErrorStrategy<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "ErrorStrategy::Stop"),
      ErrorStrategy::Skip => write!(f, "ErrorStrategy::Skip"),
      ErrorStrategy::Retry(n) => write!(f, "ErrorStrategy::Retry({})", n),
      ErrorStrategy::Custom(_) => write!(f, "ErrorStrategy::Custom"),
    }
  }
}

impl<T: std::fmt::Debug + Clone> PartialEq for ErrorStrategy<T> {
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

impl<T: std::fmt::Debug + Clone> ErrorStrategy<T> {
  pub fn new_custom<F>(f: F) -> Self
  where
    F: Fn(&StreamError<T>) -> ErrorAction + Send + Sync + 'static,
  {
    Self::Custom(Box::new(f))
  }
}

#[derive(Debug)]
pub struct StreamError<T: std::fmt::Debug + Clone> {
  pub source: Box<dyn Error + Send + Sync>,
  pub context: ErrorContext<T>,
  pub component: ComponentInfo,
  pub retries: usize,
}

impl<T: std::fmt::Debug + Clone> Clone for StreamError<T> {
  fn clone(&self) -> Self {
    Self {
      source: Box::new(StringError(self.source.to_string())),
      context: self.context.clone(),
      component: self.component.clone(),
      retries: self.retries,
    }
  }
}

#[derive(Debug)]
struct StringError(String);

impl std::fmt::Display for StringError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for StringError {}

impl<T: std::fmt::Debug + Clone> StreamError<T> {
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

impl<T: std::fmt::Debug + Clone> fmt::Display for StreamError<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Error in {} ({}): {}",
      self.component.name, self.component.type_name, self.source
    )
  }
}

impl<T: std::fmt::Debug + Clone> Error for StreamError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(self.source.as_ref())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorContext<T: std::fmt::Debug + Clone> {
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
pub struct PipelineError<T: std::fmt::Debug + Clone> {
  inner: StreamError<T>,
}

impl<T: std::fmt::Debug + Clone> PipelineError<T> {
  pub fn new<E>(error: E, context: ErrorContext<T>, component: ComponentInfo) -> Self
  where
    E: Error + Send + Sync + 'static,
    T: Send + Sync + 'static,
  {
    Self {
      inner: StreamError::new(Box::new(error), context, component),
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

impl<T: std::fmt::Debug + Clone> std::fmt::Display for PipelineError<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Pipeline error in {}: {}",
      self.inner.component.name, self.inner.source
    )
  }
}

impl<T: std::fmt::Debug + Clone> Error for PipelineError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(&*self.inner.source)
  }
}
