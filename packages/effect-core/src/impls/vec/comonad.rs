use crate::traits::comonad::Comonad;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Comonad<T> for Vec<T> {
  fn extract(self) -> T {
    // For Vec, extract returns the first element
    // This follows the comonad pattern where extract focuses on a specific element
    if self.is_empty() {
      panic!("Cannot extract from empty Vec - Vec is not a valid comonad when empty")
    } else {
      self.into_iter().next().unwrap()
    }
  }

  fn extend<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(Self) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // For Vec, extend applies the function to all possible suffixes
    // This creates a new vector where each element is transformed based on its context
    let mut result = Vec::new();
    for i in 0..=self.len() {
      let suffix = self[i..].to_vec();
      let transformed = f(suffix);
      result.push(transformed);
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_nonempty() {
    let vec = vec![1, 2, 3];
    assert_eq!(Comonad::extract(vec), 1);
  }

  #[test]
  #[should_panic(expected = "Cannot extract from empty Vec")]
  fn test_extract_empty() {
    let vec: Vec<i32> = vec![];
    let _ = Comonad::extract(vec);
  }

  #[test]
  fn test_extend_nonempty() {
    let vec = vec![1, 2, 3];
    let extended = Comonad::extend(vec, |v| v.len());
    // The result should be [3, 2, 1, 0] for suffixes [1,2,3], [2,3], [3], []
    assert_eq!(extended, vec![3, 2, 1, 0]);
  }

  #[test]
  fn test_extend_empty() {
    let vec: Vec<i32> = vec![];
    let extended = Comonad::extend(vec, |v| v.len());
    assert_eq!(extended, vec![0]);
  }

  #[test]
  fn test_duplicate_nonempty() {
    let vec = vec![1, 2, 3];
    let duplicated = Comonad::duplicate(vec);
    // The result should be a vector of vectors
    assert_eq!(duplicated.len(), 4); // [1,2,3], [2,3], [3], []
  }

  #[test]
  fn test_duplicate_empty() {
    let vec: Vec<i32> = vec![];
    let duplicated = Comonad::duplicate(vec);
    assert_eq!(duplicated, vec![vec![]]);
  }

  // Additional tests for comonad laws
  #[test]
  fn test_extend_suffix_lengths() {
    let vec = vec![1, 2, 3];
    let extended = Comonad::extend(vec, |suffix| suffix.len());
    // The result should be [3, 2, 1, 0] for suffixes [1,2,3], [2,3], [3], []
    let expected: Vec<usize> = vec![3, 2, 1, 0];
    assert_eq!(extended, expected);
  }

  #[test]
  fn test_extend_preserves_suffix_structure() {
    let vec = vec![1, 2, 3];
    let extended = Comonad::extend(vec.clone(), |suffix| suffix);
    // The first element should be the original vector
    assert_eq!(&extended[0], &vec);
    // The last element should be an empty vector
    assert_eq!(&extended[extended.len() - 1], &vec![]);
  }
}
