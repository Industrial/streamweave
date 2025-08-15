use crate::traits::comonad::Comonad;
use crate::types::threadsafe::CloneableThreadSafe;

impl Comonad<char> for String {
  fn extract(self) -> char {
    // For String, extract returns the first character
    // This follows the comonad pattern where extract focuses on a specific element
    if self.is_empty() {
      panic!("Cannot extract from empty String - String is not a valid comonad when empty")
    } else {
      self.chars().next().unwrap()
    }
  }

  fn extend<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(Self) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // For String, extend applies the function to all possible suffixes
    // This creates a new vector where each element is transformed based on its context
    let mut result = Vec::new();
    for i in 0..=self.len() {
      let substring = self[i..].to_string();
      let transformed = f(substring);
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
    let string = "hello".to_string();
    assert_eq!(Comonad::extract(string), 'h');
  }

  #[test]
  #[should_panic(expected = "Cannot extract from empty String")]
  fn test_extract_empty() {
    let string = String::new();
    let _ = Comonad::extract(string);
  }

  #[test]
  fn test_extend_nonempty() {
    let string = "abc".to_string();
    let extended = Comonad::extend(string, |s| s.len());
    // The result should be [3, 2, 1, 0] for suffixes "abc", "bc", "c", ""
    assert_eq!(extended, vec![3, 2, 1, 0]);
  }

  #[test]
  fn test_extend_empty() {
    let string = String::new();
    let extended = Comonad::extend(string, |s| s.len());
    assert_eq!(extended, vec![0]);
  }

  #[test]
  fn test_duplicate_nonempty() {
    let string = "hello".to_string();
    let duplicated = Comonad::duplicate(string);
    // The result should be a vector of strings
    assert_eq!(duplicated.len(), 6); // "hello", "ello", "llo", "lo", "o", ""
  }

  #[test]
  fn test_duplicate_empty() {
    let string = String::new();
    let duplicated = Comonad::duplicate(string);
    assert_eq!(duplicated, vec![String::new()]);
  }

  // Additional tests for comonad laws
  #[test]
  fn test_extend_suffix_lengths() {
    let string = "abc".to_string();
    let extended = Comonad::extend(string, |suffix| suffix.len());
    // The result should be [3, 2, 1, 0] for suffixes "abc", "bc", "c", ""
    let expected: Vec<usize> = vec![3, 2, 1, 0];
    assert_eq!(extended, expected);
  }

  #[test]
  fn test_extend_preserves_suffix_structure() {
    let string = "hello".to_string();
    let extended = Comonad::extend(string.clone(), |suffix| suffix);
    // The first element should be the original string
    assert_eq!(&extended[0], &string);
    // The last element should be an empty string
    assert_eq!(&extended[extended.len() - 1], "");
  }
}
