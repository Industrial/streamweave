use crate::{effect::Effect, functor::Functor};
use futures_io::Error as IoError;
use std::convert::From;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::io;

/// A natural transformation is a mapping between functors that preserves the structure.
/// It must satisfy the naturality condition: map f . transform = transform . map f
pub trait Natural<F, G>
where
  F: Functor<()>,
  G: Functor<()>,
{
  /// Transform from one functor to another while preserving structure
  fn transform<T: Clone + Send + Sync + 'static>(fa: F::HigherSelf<T>) -> G::HigherSelf<T>;
}

#[derive(Debug)]
struct ClonableIoError(IoError);

impl Clone for ClonableIoError {
  fn clone(&self) -> Self {
    ClonableIoError(IoError::new(self.0.kind(), self.0.to_string()))
  }
}

impl fmt::Display for ClonableIoError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Error for ClonableIoError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    self.0.source()
  }
}

impl From<IoError> for ClonableIoError {
  fn from(err: IoError) -> Self {
    ClonableIoError(err)
  }
}

impl From<ClonableIoError> for IoError {
  fn from(err: ClonableIoError) -> Self {
    err.0
  }
}

// Vec -> Option (single element vector)
impl Natural<Vec<()>, Option<()>> for Option<()> {
  fn transform<T: Clone + Send + Sync + 'static>(fa: Vec<T>) -> Option<T> {
    fa.into_iter().next()
  }
}

// Vec -> Result (single element vector)
impl<E> Natural<Vec<()>, Result<(), E>> for Result<(), E>
where
  E: Error + Send + Sync + From<std::io::Error> + 'static,
{
  fn transform<T: Clone + Send + Sync + 'static>(fa: Vec<T>) -> Result<T, E> {
    fa.into_iter().next().ok_or_else(|| {
      E::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Empty vector",
      ))
    })
  }
}

// Option -> Vec
impl Natural<Option<()>, Vec<()>> for Vec<()> {
  fn transform<T: Clone + Send + Sync + 'static>(fa: Option<T>) -> Vec<T> {
    fa.map(|x| vec![x]).unwrap_or_default()
  }
}

// Result -> Vec
impl<E> Natural<Result<(), E>, Vec<()>> for Vec<()>
where
  E: Error + Send + Sync + 'static,
{
  fn transform<T: Clone + Send + Sync + 'static>(fa: Result<T, E>) -> Vec<T> {
    fa.map(|x| vec![x]).unwrap_or_default()
  }
}

// Result -> Option
impl<E> Natural<Result<(), E>, Option<()>> for Option<()>
where
  E: Error + Send + Sync + 'static,
{
  fn transform<T: Clone + Send + Sync + 'static>(fa: Result<T, E>) -> Option<T> {
    fa.ok()
  }
}

// Option -> Result
impl<E> Natural<Option<()>, Result<(), E>> for Result<(), E>
where
  E: Error + Send + Sync + From<std::io::Error> + 'static,
{
  fn transform<T: Clone + Send + Sync + 'static>(fa: Option<T>) -> Result<T, E> {
    fa.ok_or_else(|| E::from(std::io::Error::new(std::io::ErrorKind::Other, "None value")))
  }
}

// Example implementation: Option -> Effect
impl<E> Natural<Option<()>, Effect<(), E>> for Effect<(), E>
where
  E: Error + Send + Sync + From<std::io::Error> + 'static,
{
  fn transform<T: Clone + Send + Sync + 'static>(fa: Option<T>) -> Effect<T, E> {
    match fa {
      Some(value) => Effect::pure(value),
      None => Effect::error(E::from(std::io::Error::new(
        std::io::ErrorKind::Other,
        "None value",
      ))),
    }
  }
}

// Example implementation: Result -> Effect
impl<E: Error + Send + Sync + 'static> Natural<Result<(), E>, Effect<(), E>> for Effect<(), E> {
  fn transform<T: Clone + Send + Sync + 'static>(fa: Result<T, E>) -> Effect<T, E> {
    match fa {
      Ok(value) => Effect::pure(value),
      Err(e) => Effect::error(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::io::Error as IoError;

  // Helper function to create test error
  fn test_error(msg: &str) -> IoError {
    IoError::new(std::io::ErrorKind::Other, msg)
  }

  // Helper function to compare errors
  fn assert_errors_eq(err1: &IoError, err2: &IoError) {
    assert_eq!(err1.kind(), err2.kind());
    assert_eq!(err1.to_string(), err2.to_string());
  }

  // Helper function to compare results
  fn assert_results_eq<T: PartialEq + std::fmt::Debug>(
    res1: Result<T, IoError>,
    res2: Result<T, IoError>,
  ) {
    match (res1, res2) {
      (Ok(v1), Ok(v2)) => assert_eq!(v1, v2),
      (Err(e1), Err(e2)) => assert_eq!(e1.to_string(), e2.to_string()),
      _ => panic!("Results are not equal"),
    }
  }

  #[test]
  fn test_vec_to_option() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      // Non-empty vector
      let vec = vec![x];
      let opt = <Option<()> as Natural<Vec<()>, Option<()>>>::transform(vec.clone());
      assert_eq!(opt, Some(x));

      // Empty vector
      let empty: Vec<i32> = vec![];
      let opt_empty = <Option<()> as Natural<Vec<()>, Option<()>>>::transform(empty);
      assert_eq!(opt_empty, None);
    });
  }

  #[test]
  fn test_vec_to_result() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      // Non-empty vector
      let vec = vec![x];
      let res = <Result<(), IoError> as Natural<Vec<()>, Result<(), IoError>>>::transform(vec.clone());
      assert_results_eq(res, Ok(x));

      // Empty vector
      let empty: Vec<i32> = vec![];
      let res_empty = <Result<(), IoError> as Natural<Vec<()>, Result<(), IoError>>>::transform(empty);
      assert!(res_empty.is_err());
    });
  }

  #[test]
  fn test_option_to_vec() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      // Some value
      let opt = Some(x);
      let vec = <Vec<()> as Natural<Option<()>, Vec<()>>>::transform(opt);
      assert_eq!(vec, vec![x]);

      // None value
      let none: Option<i32> = None;
      let vec_none = <Vec<()> as Natural<Option<()>, Vec<()>>>::transform(none);
      assert!(vec_none.is_empty());
    });
  }

  #[test]
  fn test_result_to_vec() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      // Ok value
      let res: Result<i32, IoError> = Ok(x);
      let vec = <Vec<()> as Natural<Result<(), IoError>, Vec<()>>>::transform(res);
      assert_eq!(vec, vec![x]);

      // Err value
      let err = test_error("test error");
      let res_err: Result<i32, IoError> = Err(err);
      let vec_err = <Vec<()> as Natural<Result<(), IoError>, Vec<()>>>::transform(res_err);
      assert!(vec_err.is_empty());
    });
  }

  #[test]
  fn test_result_to_option() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      // Ok value
      let res: Result<i32, IoError> = Ok(x);
      let opt = <Option<()> as Natural<Result<(), IoError>, Option<()>>>::transform(res);
      assert_eq!(opt, Some(x));

      // Err value
      let err = test_error("test error");
      let res_err: Result<i32, IoError> = Err(err);
      let opt_err = <Option<()> as Natural<Result<(), IoError>, Option<()>>>::transform(res_err);
      assert_eq!(opt_err, None);
    });
  }

  #[test]
  fn test_option_to_result() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      // Some value
      let opt = Some(x);
      let res = <Result<(), IoError> as Natural<Option<()>, Result<(), IoError>>>::transform(opt);
      assert_results_eq(res, Ok(x));

      // None value
      let none: Option<i32> = None;
      let res_none = <Result<(), IoError> as Natural<Option<()>, Result<(), IoError>>>::transform(none);
      assert!(res_none.is_err());
    });
  }

  #[test]
  fn test_option_to_effect_some() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(async {
        let opt = Some(x);
        let eff = <Effect<_, IoError> as Natural<Option<()>, Effect<(), IoError>>>::transform(opt);
        assert_eq!(eff.run().await.unwrap(), x);
      });
    });
  }

  #[test]
  fn test_option_to_effect_none() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let opt: Option<i32> = None;
      let eff = <Effect<_, IoError> as Natural<Option<()>, Effect<(), IoError>>>::transform(opt);
      assert!(eff.run().await.is_err());
    });
  }

  #[test]
  fn test_result_to_effect_ok() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(async {
        let res: Result<i32, IoError> = Ok(x);
        let eff = <Effect<_, IoError> as Natural<Result<(), IoError>, Effect<(), IoError>>>::transform(res);
        assert_eq!(eff.run().await.unwrap(), x);
      });
    });
  }

  #[test]
  fn test_result_to_effect_err() {
    let strategy = ".*".prop_filter("Empty string is not allowed", |s| !s.is_empty());
    proptest!(|(msg in strategy)| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(async {
        let err = test_error(&msg);
        let clonable_err = ClonableIoError(err);
        let res: Result<i32, IoError> = Err(clonable_err.clone().0);
        let eff = <Effect<_, IoError> as Natural<Result<(), IoError>, Effect<(), IoError>>>::transform(res);
        let err2 = eff.run().await.unwrap_err();
        assert_errors_eq(&err2, &clonable_err.0);
      });
    });
  }

  // Naturality condition tests for new implementations
  #[test]
  fn test_naturality_vec_to_option() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let f = |x: i32| x * 2;

      // Path 1: map f . transform
      let vec = vec![x];
      let opt1 = <Option<()> as Natural<Vec<()>, Option<()>>>::transform(vec);
      let mapped1 = opt1.map(f);

      // Path 2: transform . map f
      let vec2 = vec![x];
      let mapped_vec = vec2.into_iter().map(f).collect::<Vec<_>>();
      let opt2 = <Option<()> as Natural<Vec<()>, Option<()>>>::transform(mapped_vec);

      assert_eq!(mapped1, opt2);
    });
  }

  #[test]
  fn test_naturality_vec_to_result() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let f = |x: i32| x * 2;

      // Path 1: map f . transform
      let vec = vec![x];
      let res1 = <Result<(), IoError> as Natural<Vec<()>, Result<(), IoError>>>::transform(vec);
      let mapped1 = res1.map(f);

      // Path 2: transform . map f
      let vec2 = vec![x];
      let mapped_vec = vec2.into_iter().map(f).collect::<Vec<_>>();
      let res2 = <Result<(), IoError> as Natural<Vec<()>, Result<(), IoError>>>::transform(mapped_vec);

      assert_results_eq(mapped1, res2);
    });
  }

  #[test]
  fn test_naturality_option_to_vec() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let f = |x: i32| x * 2;

      // Path 1: map f . transform
      let opt = Some(x);
      let vec1 = <Vec<()> as Natural<Option<()>, Vec<()>>>::transform(opt);
      let mapped1 = vec1.into_iter().map(f).collect::<Vec<_>>();

      // Path 2: transform . map f
      let opt2 = Some(x);
      let mapped_opt = opt2.map(f);
      let vec2 = <Vec<()> as Natural<Option<()>, Vec<()>>>::transform(mapped_opt);

      assert_eq!(mapped1, vec2);
    });
  }

  #[test]
  fn test_naturality_result_to_vec() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let f = |x: i32| x * 2;

      // Path 1: map f . transform
      let res: Result<i32, IoError> = Ok(x);
      let vec1 = <Vec<()> as Natural<Result<(), IoError>, Vec<()>>>::transform(res);
      let mapped1 = vec1.into_iter().map(f).collect::<Vec<_>>();

      // Path 2: transform . map f
      let res2: Result<i32, IoError> = Ok(x);
      let mapped_res = res2.map(f);
      let vec2 = <Vec<()> as Natural<Result<(), IoError>, Vec<()>>>::transform(mapped_res);

      assert_eq!(mapped1, vec2);
    });
  }

  #[test]
  fn test_naturality_result_to_option() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let f = |x: i32| x * 2;

      // Path 1: map f . transform
      let res: Result<i32, IoError> = Ok(x);
      let opt1 = <Option<()> as Natural<Result<(), IoError>, Option<()>>>::transform(res);
      let mapped1 = opt1.map(f);

      // Path 2: transform . map f
      let res2: Result<i32, IoError> = Ok(x);
      let mapped_res = res2.map(f);
      let opt2 = <Option<()> as Natural<Result<(), IoError>, Option<()>>>::transform(mapped_res);

      assert_eq!(mapped1, opt2);
    });
  }

  #[test]
  fn test_naturality_option_to_result() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let f = |x: i32| x * 2;

      // Path 1: map f . transform
      let opt = Some(x);
      let res1 = <Result<(), IoError> as Natural<Option<()>, Result<(), IoError>>>::transform(opt);
      let mapped1 = res1.map(f);

      // Path 2: transform . map f
      let opt2 = Some(x);
      let mapped_opt = opt2.map(f);
      let res2 = <Result<(), IoError> as Natural<Option<()>, Result<(), IoError>>>::transform(mapped_opt);

      assert_results_eq(mapped1, res2);
    });
  }

  #[test]
  fn test_naturality_condition_option() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(async {
        // Test that map f . transform = transform . map f
        let f = |x: i32| x * 2;

        // Path 1: map f . transform
        let opt = Some(x);
        let eff1 = <Effect<_, IoError> as Natural<Option<()>, Effect<(), IoError>>>::transform(opt);
        let mapped1 = eff1.map(f);

        // Path 2: transform . map f
        let opt2 = Some(x);
        let mapped_opt = opt2.map(f);
        let eff2 = <Effect<_, IoError> as Natural<Option<()>, Effect<(), IoError>>>::transform(mapped_opt);

        let res1 = mapped1.run().await;
        let res2 = eff2.run().await;
        assert_results_eq(res1, res2);
      });
    });
  }

  #[test]
  fn test_naturality_condition_result() {
    let strategy = any::<i32>();
    proptest!(|(x in strategy)| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(async {
        // Test that map f . transform = transform . map f
        let f = |x: i32| x * 2;

        // Path 1: map f . transform
        let res: Result<i32, IoError> = Ok(x);
        let eff1 = <Effect<_, IoError> as Natural<Result<(), IoError>, Effect<(), IoError>>>::transform(res);
        let mapped1 = eff1.map(f);

        // Path 2: transform . map f
        let res2: Result<i32, IoError> = Ok(x);
        let mapped_res = res2.map(f);
        let eff2 = <Effect<_, IoError> as Natural<Result<(), IoError>, Effect<(), IoError>>>::transform(mapped_res);

        let res1 = mapped1.run().await;
        let res2 = eff2.run().await;
        assert_results_eq(res1, res2);
      });
    });
  }
}
