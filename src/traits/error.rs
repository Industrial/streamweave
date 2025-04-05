use std::error::Error as StdError;

/// Trait for components that can produce errors
pub trait Error {
  type Error: StdError + Send + Sync + 'static;
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fmt;

  // Test error implementation
  #[derive(Debug)]
  struct TestError(String);

  impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl StdError for TestError {}

  // Test component that produces errors
  struct TestComponent;

  impl Error for TestComponent {
    type Error = TestError;
  }

  #[test]
  fn test_error_constraints() {
    fn assert_error<T: Error>()
    where
      T::Error: StdError + Send + Sync,
    {
    }

    assert_error::<TestComponent>();
  }

  #[test]
  fn test_error_static_dispatch() {
    fn takes_error_type<T: Error>(_: T)
    where
      T::Error: StdError + Send + Sync,
    {
    }

    takes_error_type(TestComponent);
  }

  #[test]
  fn test_error_type_usage() {
    let _: <TestComponent as Error>::Error = TestError("test".to_string());
  }
}
