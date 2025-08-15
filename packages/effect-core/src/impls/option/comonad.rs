use crate::traits::comonad::Comonad;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Comonad<T> for Option<T> {
  fn extract(self) -> T {
    // For Option, extract returns the value if Some, or panics if None
    // This follows the comonad pattern where extract should always succeed
    self.expect("Cannot extract from None - Option is not a valid comonad when None")
  }

  fn extend<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(Self) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // For Option, extend applies the function to the entire Option structure
    // If None, return None (preserving the structure)
    // If Some, apply the function to the Some value
    match self {
      Some(_) => Some(f(self)),
      None => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_some() {
    let option = Some(42);
    assert_eq!(Comonad::extract(option), 42);
  }

  #[test]
  #[should_panic(expected = "Cannot extract from None")]
  fn test_extract_none() {
    let option: Option<i32> = None;
    let _ = Comonad::extract(option);
  }

  #[test]
  fn test_extend_some() {
    let option = Some(42);
    let result = Comonad::extend(option, |opt| opt.map(|x| x * 2));
    assert_eq!(result, Some(Some(84)));
  }

  #[test]
  fn test_extend_none() {
    let option: Option<i32> = None;
    let result = Comonad::extend(option, |opt| opt.map(|x| x * 2));
    assert_eq!(result, None);
  }

  #[test]
  fn test_duplicate_some() {
    let option = Some(42);
    let duplicated = Comonad::duplicate(option);
    assert_eq!(duplicated, Some(Some(42)));
  }

  #[test]
  fn test_duplicate_none() {
    let option: Option<i32> = None;
    let duplicated = Comonad::duplicate(option);
    assert_eq!(duplicated, None);
  }

  // Additional tests for comonad laws
  #[test]
  fn test_left_identity_law() {
    // Left identity law: comonad.extend(|w| w.extract()) == comonad
    let comonad = Some(42);
    let result = Comonad::extend(comonad.clone(), |w| Comonad::extract(w));
    assert_eq!(result, Some(42));
  }

  #[test]
  fn test_right_identity_law() {
    // Right identity law: comonad.extract() == comonad.extend(|w| w.extract()).extract()
    let comonad = Some(42);
    let extracted = Comonad::extract(comonad.clone());
    let extended = Comonad::extend(comonad, |w| Comonad::extract(w));
    let result = Comonad::extract(extended);
    assert_eq!(extracted, result);
  }

  #[test]
  fn test_associativity_law() {
    // Associativity law: comonad.extend(f).extend(g) == comonad.extend(|w| g(w.extend(f)))
    let comonad = Some(42);

    // Define extension functions that work with the nested structure
    let f = |w: Option<i32>| w.map(|val| val.wrapping_mul(2));
    let g = |w: Option<Option<i32>>| w.flatten().map(|val| val.wrapping_add(10));

    // Apply extensions sequentially
    let result1 = Comonad::extend(comonad.clone(), f);
    let result1 = Comonad::extend(result1, g);

    // Apply composed extension
    let result2 = Comonad::extend(comonad, move |w| {
      g(Comonad::extend(w, f))
    });

    assert_eq!(result1, result2);
  }
} 