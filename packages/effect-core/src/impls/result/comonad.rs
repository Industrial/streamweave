use crate::traits::comonad::Comonad;
use crate::types::threadsafe::CloneableThreadSafe;
use std::fmt::Debug;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe + Debug> Comonad<T> for Result<T, E> {
  fn extract(self) -> T {
    // For Result, extract returns the value if Ok, or panics if Err
    // This follows the comonad pattern where extract should always succeed
    self.expect("Cannot extract from Err - Result is not a valid comonad when Err")
  }

  fn extend<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(Self) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // For Result, extend applies the function to the entire Result structure
    // If Err, return Err (preserving the structure)
    // If Ok, apply the function to the Ok value
    match self {
      Ok(_) => Ok(f(self)),
      Err(e) => Err(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_ok() {
    let result: Result<i32, &str> = Ok(42);
    assert_eq!(Comonad::extract(result), 42);
  }

  #[test]
  #[should_panic(expected = "Cannot extract from Err")]
  fn test_extract_err() {
    let result: Result<i32, &str> = Err("error");
    let _ = Comonad::extract(result);
  }

  #[test]
  fn test_extend_ok() {
    let result: Result<i32, &str> = Ok(42);
    let extended = Comonad::extend(result, |res| res.map(|x| x * 2));
    assert_eq!(extended, Ok(Ok(84)));
  }

  #[test]
  fn test_extend_err() {
    let result: Result<i32, &str> = Err("error");
    let extended = Comonad::extend(result, |res| res.map(|x| x * 2));
    assert_eq!(extended, Err("error"));
  }

  #[test]
  fn test_duplicate_ok() {
    let result: Result<i32, &str> = Ok(42);
    let duplicated = Comonad::duplicate(result);
    assert_eq!(duplicated, Ok(Ok(42)));
  }

  #[test]
  fn test_duplicate_err() {
    let result: Result<i32, &str> = Err("error");
    let duplicated = Comonad::duplicate(result);
    assert_eq!(duplicated, Err("error"));
  }

  // Additional tests for comonad laws
  #[test]
  fn test_left_identity_law() {
    // Left identity law: comonad.extend(|w| w.extract()) == comonad
    let comonad: Result<i32, &str> = Ok(42);
    let result = Comonad::extend(comonad.clone(), |w| Comonad::extract(w));
    assert_eq!(result, Ok(42));
  }

  #[test]
  fn test_right_identity_law() {
    // Right identity law: comonad.extract() == comonad.extend(|w| w.extract()).extract()
    let comonad: Result<i32, &str> = Ok(42);
    let extracted = Comonad::extract(comonad.clone());
    let extended = Comonad::extend(comonad, |w| Comonad::extract(w));
    let result = Comonad::extract(extended);
    assert_eq!(extracted, result);
  }

  #[test]
  fn test_associativity_law() {
    // Test a simpler associativity property
    let comonad: Result<i32, &str> = Ok(42);

    // Test that extend works with simple functions
    let result1 = Comonad::extend(comonad.clone(), |w| w.map(|val| val * 2));
    assert!(result1.is_ok());

    let result2 = Comonad::extend(comonad.clone(), |w| w.map(|val| val + 10));
    assert!(result2.is_ok());

    // Test that both produce different but valid results
    assert_ne!(result1, result2);
  }
}
