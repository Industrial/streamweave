use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorAction {
  Stop,
  Skip,
  Retry,
}

pub enum ErrorStrategy<T: std::fmt::Debug + Clone + Send + Sync> {
  Stop,
  Skip,
  Retry(usize),
  Custom(Arc<dyn Fn(&StreamError<T>) -> ErrorAction + Send + Sync>),
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Clone for ErrorStrategy<T> {
  fn clone(&self) -> Self {
    match self {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) => ErrorStrategy::Retry(*n),
      ErrorStrategy::Custom(handler) => ErrorStrategy::Custom(handler.clone()),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> fmt::Debug for ErrorStrategy<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "ErrorStrategy::Stop"),
      ErrorStrategy::Skip => write!(f, "ErrorStrategy::Skip"),
      ErrorStrategy::Retry(n) => write!(f, "ErrorStrategy::Retry({})", n),
      ErrorStrategy::Custom(_) => write!(f, "ErrorStrategy::Custom"),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> PartialEq for ErrorStrategy<T> {
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

impl<T: std::fmt::Debug + Clone + Send + Sync> ErrorStrategy<T> {
  pub fn new_custom<F>(f: F) -> Self
  where
    F: Fn(&StreamError<T>) -> ErrorAction + Send + Sync + 'static,
  {
    Self::Custom(Arc::new(f))
  }
}

#[derive(Debug)]
pub struct StreamError<T: std::fmt::Debug + Clone + Send + Sync> {
  pub source: Box<dyn Error + Send + Sync>,
  pub context: ErrorContext<T>,
  pub component: ComponentInfo,
  pub retries: usize,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Clone for StreamError<T> {
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

impl<T: std::fmt::Debug + Clone + Send + Sync> StreamError<T> {
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

impl<T: std::fmt::Debug + Clone + Send + Sync> fmt::Display for StreamError<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Error in {} ({}): {}",
      self.component.name, self.component.type_name, self.source
    )
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Error for StreamError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(self.source.as_ref())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorContext<T: std::fmt::Debug + Clone + Send + Sync> {
  pub timestamp: chrono::DateTime<chrono::Utc>,
  pub item: Option<T>,
  pub component_name: String,
  pub component_type: String,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Default for ErrorContext<T> {
  fn default() -> Self {
    Self {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "default".to_string(),
      component_type: "default".to_string(),
    }
  }
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

impl Default for ComponentInfo {
  fn default() -> Self {
    Self {
      name: "default".to_string(),
      type_name: "default".to_string(),
    }
  }
}

impl ComponentInfo {
  pub fn new(name: String, type_name: String) -> Self {
    Self { name, type_name }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineErrorContext<T: std::fmt::Debug + Clone + Send + Sync> {
  pub context: ErrorContext<T>,
  pub stage: PipelineStage,
}

#[derive(Debug)]
pub struct PipelineError<T: std::fmt::Debug + Clone + Send + Sync> {
  inner: StreamError<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> PipelineError<T> {
  pub fn new<E>(error: E, context: ErrorContext<T>, component: ComponentInfo) -> Self
  where
    E: Error + Send + Sync + 'static,
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

impl<T: std::fmt::Debug + Clone + Send + Sync> std::fmt::Display for PipelineError<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Pipeline error in {}: {}",
      self.inner.component.name, self.inner.source
    )
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Error for PipelineError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(&*self.inner.source)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::error::Error;
  use std::fmt;

  #[test]
  fn test_error_action() {
    assert_eq!(ErrorAction::Stop, ErrorAction::Stop);
    assert_eq!(ErrorAction::Skip, ErrorAction::Skip);
    assert_eq!(ErrorAction::Retry, ErrorAction::Retry);
    assert_ne!(ErrorAction::Stop, ErrorAction::Skip);
    assert_ne!(ErrorAction::Skip, ErrorAction::Retry);
    assert_ne!(ErrorAction::Retry, ErrorAction::Stop);
  }

  #[test]
  fn test_error_strategy_clone() {
    let strategy = ErrorStrategy::<i32>::Stop;
    assert_eq!(strategy.clone(), ErrorStrategy::Stop);

    let strategy = ErrorStrategy::<i32>::Skip;
    assert_eq!(strategy.clone(), ErrorStrategy::Skip);

    let strategy = ErrorStrategy::<i32>::Retry(3);
    assert_eq!(strategy.clone(), ErrorStrategy::Retry(3));

    let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
    let cloned = strategy.clone();
    let _error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    assert_eq!(strategy, cloned);
  }

  #[test]
  fn test_error_strategy_debug() {
    assert_eq!(
      format!("{:?}", ErrorStrategy::<i32>::Stop),
      "ErrorStrategy::Stop"
    );
    assert_eq!(
      format!("{:?}", ErrorStrategy::<i32>::Skip),
      "ErrorStrategy::Skip"
    );
    assert_eq!(
      format!("{:?}", ErrorStrategy::<i32>::Retry(3)),
      "ErrorStrategy::Retry(3)"
    );
    assert_eq!(
      format!(
        "{:?}",
        ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip)
      ),
      "ErrorStrategy::Custom"
    );
  }

  #[test]
  fn test_error_strategy_partial_eq() {
    assert_eq!(ErrorStrategy::<i32>::Stop, ErrorStrategy::Stop);
    assert_eq!(ErrorStrategy::<i32>::Skip, ErrorStrategy::Skip);
    assert_eq!(ErrorStrategy::<i32>::Retry(3), ErrorStrategy::Retry(3));
    assert_ne!(ErrorStrategy::<i32>::Stop, ErrorStrategy::Skip);
    assert_ne!(ErrorStrategy::<i32>::Skip, ErrorStrategy::Retry(3));
    assert_ne!(ErrorStrategy::<i32>::Retry(3), ErrorStrategy::Stop);
  }

  #[test]
  fn test_error_strategy_new_custom() {
    let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
    let error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    if let ErrorStrategy::Custom(handler) = strategy {
      assert_eq!(handler(&error), ErrorAction::Skip);
    } else {
      panic!("Expected Custom variant");
    }
  }

  #[test]
  fn test_stream_error_clone() {
    let error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    let cloned = error.clone();
    assert_eq!(error.source.to_string(), cloned.source.to_string());
    assert_eq!(error.context, cloned.context);
    assert_eq!(error.component, cloned.component);
    assert_eq!(error.retries, cloned.retries);
  }

  #[test]
  fn test_stream_error_display() {
    let error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
      retries: 0,
    };
    assert_eq!(
      error.to_string(),
      "Error in test (TestComponent): test error"
    );
  }

  #[test]
  fn test_stream_error_error() {
    let source = StringError("test error".to_string());
    let error: StreamError<i32> = StreamError {
      source: Box::new(source),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    assert_eq!(error.source().unwrap().to_string(), "test error");
  }

  #[test]
  fn test_stream_error_new() {
    let source = StringError("test error".to_string());
    let context: ErrorContext<i32> = ErrorContext::default();
    let component = ComponentInfo::default();
    let error = StreamError::new(Box::new(source), context.clone(), component.clone());
    assert_eq!(error.source.to_string(), "test error");
    assert_eq!(error.context, context);
    assert_eq!(error.component, component);
    assert_eq!(error.retries, 0);
  }

  #[test]
  fn test_error_context_default() {
    let context = ErrorContext::<i32>::default();
    assert!(context.timestamp <= chrono::Utc::now());
    assert_eq!(context.item, None);
    assert_eq!(context.component_name, "default");
    assert_eq!(context.component_type, "default");
  }

  #[test]
  fn test_error_context_clone() {
    let context = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let cloned = context.clone();
    assert_eq!(context.timestamp, cloned.timestamp);
    assert_eq!(context.item, cloned.item);
    assert_eq!(context.component_name, cloned.component_name);
    assert_eq!(context.component_type, cloned.component_type);
  }

  #[test]
  fn test_error_context_partial_eq() {
    let context1 = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let context2 = ErrorContext {
      timestamp: context1.timestamp,
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    assert_eq!(context1, context2);
  }

  #[test]
  fn test_component_info_default() {
    let info = ComponentInfo::default();
    assert_eq!(info.name, "default");
    assert_eq!(info.type_name, "default");
  }

  #[test]
  fn test_component_info_clone() {
    let info = ComponentInfo {
      name: "test".to_string(),
      type_name: "TestComponent".to_string(),
    };
    let cloned = info.clone();
    assert_eq!(info.name, cloned.name);
    assert_eq!(info.type_name, cloned.type_name);
  }

  #[test]
  fn test_component_info_partial_eq() {
    let info1 = ComponentInfo {
      name: "test".to_string(),
      type_name: "TestComponent".to_string(),
    };
    let info2 = ComponentInfo {
      name: "test".to_string(),
      type_name: "TestComponent".to_string(),
    };
    assert_eq!(info1, info2);
  }

  #[test]
  fn test_component_info_new() {
    let info = ComponentInfo::new("test".to_string(), "TestComponent".to_string());
    assert_eq!(info.name, "test");
    assert_eq!(info.type_name, "TestComponent");
  }

  #[test]
  fn test_pipeline_error_new() {
    let source = StringError("test error".to_string());
    let context: ErrorContext<i32> = ErrorContext::default();
    let component = ComponentInfo::default();
    let error: PipelineError<i32> = PipelineError::new(source, context.clone(), component.clone());
    assert_eq!(error.context(), &context);
    assert_eq!(error.component(), &component);
  }

  #[test]
  fn test_pipeline_error_from_stream_error() {
    let stream_error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    let error: PipelineError<i32> = PipelineError::from_stream_error(stream_error.clone());
    assert_eq!(error.context(), &stream_error.context);
    assert_eq!(error.component(), &stream_error.component);
  }

  #[test]
  fn test_pipeline_error_display() {
    let error: PipelineError<i32> = PipelineError::new(
      StringError("test error".to_string()),
      ErrorContext::default(),
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );
    assert_eq!(error.to_string(), "Pipeline error in test: test error");
  }

  #[test]
  fn test_pipeline_error_error() {
    let source = StringError("test error".to_string());
    let error: PipelineError<i32> =
      PipelineError::new(source, ErrorContext::default(), ComponentInfo::default());
    assert_eq!(error.source().unwrap().to_string(), "test error");
  }
}
