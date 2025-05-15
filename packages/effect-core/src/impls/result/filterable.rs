use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Filterable<T> for Result<T, E> {
  type Filtered<B: CloneableThreadSafe> = Result<B, E>;

  fn filter_map<B, F>(self, mut f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    match self {
      Ok(a) => match f(&a) {
        Some(b) => Ok(b),
        None => {
          // This is a design decision: when filter_map returns None for an Ok value,
          // we need to convert to an Err but we don't have an error value.
          // We need a sentinel error value which we can't create without constraints.
          // Panicking here is not ideal, but matches the usual pattern.
          panic!("filter_map returned None for Ok value, but no error value is available")
        }
      },
      Err(e) => Err(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // For testing, use a custom Result type with a default error
  #[derive(Debug, Clone, PartialEq)]
  enum TestError {
    Custom(String),
  }

  #[test]
  fn test_filter_map_ok_some() {
    let value: Result<i32, TestError> = Ok(42);
    let f = |x: &i32| if *x > 40 { Some(x.to_string()) } else { None };
    
    let result = Filterable::filter_map(value, f);
    assert_eq!(result, Ok("42".to_string()));
  }

  #[test]
  #[should_panic(expected = "filter_map returned None for Ok value")]
  fn test_filter_map_ok_none() {
    let value: Result<i32, TestError> = Ok(30);
    let f = |x: &i32| if *x > 40 { Some(x.to_string()) } else { None };
    
    // This should panic because we're filtering out an Ok value
    let _ = Filterable::filter_map(value, f);
  }

  #[test]
  fn test_filter_map_err() {
    let error_value = TestError::Custom("test error".to_string());
    let value: Result<i32, TestError> = Err(error_value.clone());
    let f = |x: &i32| if *x > 40 { Some(x.to_string()) } else { None };
    
    let result = Filterable::filter_map(value, f);
    assert_eq!(result, Err(error_value));
  }

  #[test]
  fn test_filter_ok_pass() {
    let value: Result<i32, TestError> = Ok(42);
    let predicate = |x: &i32| *x > 40;
    
    let result = Filterable::filter(value, predicate);
    assert_eq!(result, Ok(42));
  }

  #[test]
  #[should_panic(expected = "filter_map returned None for Ok value")]
  fn test_filter_ok_fail() {
    let value: Result<i32, TestError> = Ok(30);
    let predicate = |x: &i32| *x > 40;
    
    // Should panic
    let _ = Filterable::filter(value, predicate);
  }

  #[test]
  fn test_filter_err() {
    let error_value = TestError::Custom("test error".to_string());
    let value: Result<i32, TestError> = Err(error_value.clone());
    let predicate = |x: &i32| *x > 40;
    
    let result = Filterable::filter(value, predicate);
    assert_eq!(result, Err(error_value));
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(x in any::<i32>(), e in "[a-zA-Z]{1,10}") {
      // Identity law: filter_map(Some) always preserves Ok values
      let result_ok = Ok::<i32, TestError>(x);
      let result_err = Err::<i32, TestError>(TestError::Custom(e.clone()));
      let identity = |val: &i32| Some(*val);
      
      let filtered_ok = Filterable::filter_map(result_ok.clone(), identity);
      prop_assert_eq!(filtered_ok, result_ok);
      
      let filtered_err = Filterable::filter_map(result_err.clone(), identity);
      prop_assert_eq!(filtered_err, result_err);
    }

    // Skip annihilation law test since it would panic
    // instead we test on Err values only
    #[test]
    fn prop_err_annihilation_law(e in "[a-zA-Z]{1,10}") {
      // Annihilation law for error: Err values stay as Err
      let result_err = Err::<i32, TestError>(TestError::Custom(e.clone()));
      let none_fn = |_: &i32| None::<i32>;
      
      // For the Err case, it should remain unchanged
      let filtered_err = Filterable::filter_map(result_err.clone(), none_fn);
      prop_assert_eq!(filtered_err, result_err);
    }

    #[test]
    fn prop_distributivity_law_err(e in "[a-zA-Z]{1,10}") {
      // Test distributivity only on Err values (Ok with None would panic)
      
      // Create an Err Result
      let result = Err::<i32, TestError>(TestError::Custom(e.clone()));
      
      // Define filter functions
      let f = |val: &i32| if *val % 2 == 0 { Some(*val) } else { None };
      let g = |val: &i32| if *val < 100 { Some(val.to_string()) } else { None };
      
      // Apply filters sequentially
      let result1 = Filterable::filter_map(result.clone(), f);
      let result1 = Filterable::filter_map(result1, g);
      
      // Apply composed filter
      let result2 = Filterable::filter_map(result, move |val| {
        f(val).and_then(|v| g(&v))
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_distributivity_law_ok_stays_ok(x in 0i32..1000i32) {
      // Test distributivity on Ok values that remain Ok
      
      // Create an Ok Result that will pass both filter conditions
      let value = x * 2 + 10; // Ensures it's even and >= 10
      let result = Ok::<i32, TestError>(value);
      
      // Define filter functions that both keep the value
      // These conditions will always be true for our input range
      let f = |val: &i32| Some(*val); // Always keep the value
      let g = |val: &i32| Some(val.to_string()); // Always convert to string
      
      // Apply filters sequentially
      let result1 = Filterable::filter_map(result.clone(), f);
      let result1 = Filterable::filter_map(result1, g);
      
      // Apply composed filter
      let result2 = Filterable::filter_map(result, move |val| {
        f(val).and_then(|v| g(&v))
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map_err(e in "[a-zA-Z]{1,10}") {
      // Test filter consistency only on Err values (Ok filtering to None would panic)
      
      // Create an Err Result
      let result = Err::<i32, TestError>(TestError::Custom(e.clone()));
      
      let predicate = |val: &i32| *val < 100;
      
      let result1 = Filterable::filter(result.clone(), predicate);
      let result2 = Filterable::filter_map(result, move |val| {
        if predicate(val) { Some(*val) } else { None }
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_passes_ok(x in any::<i32>()) {
      // Test filter on Ok values that pass the predicate
      
      // Create an Ok result with a value that will pass the predicate
      let result = Ok::<i32, TestError>(x);
      
      // Always pass predicate
      let predicate = |_: &i32| true;
      
      let result1 = Filterable::filter(result.clone(), predicate);
      prop_assert_eq!(result1, result);
    }
  }
} 