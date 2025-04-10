use crate::monad::Monad;
use std::fmt::Debug;
use std::str::FromStr;

/// A type that represents a string that is guaranteed to be non-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString {
  inner: String,
}

impl NonEmptyString {
  /// Creates a new NonEmptyString from a String.
  /// Returns None if the string is empty.
  pub fn new(s: String) -> Option<Self> {
    if s.is_empty() {
      None
    } else {
      Some(Self { inner: s })
    }
  }

  /// Creates a new NonEmptyString from a &str.
  /// Returns None if the string is empty.
  pub fn try_from_str(s: &str) -> Option<Self> {
    if s.is_empty() {
      None
    } else {
      Some(Self {
        inner: s.to_string(),
      })
    }
  }

  /// Returns the length of the string.
  pub fn len(&self) -> usize {
    self.inner.len()
  }

  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Returns true if the string contains only ASCII characters.
  pub fn is_ascii(&self) -> bool {
    self.inner.is_ascii()
  }

  /// Returns the first character of the string.
  pub fn first_char(&self) -> char {
    self.inner.chars().next().unwrap()
  }

  /// Returns the string as a &str.
  pub fn as_str(&self) -> &str {
    &self.inner
  }

  /// Converts the NonEmptyString into a String.
  pub fn into_string(self) -> String {
    self.inner
  }

  /// Maps a function over the characters of the string.
  pub fn map_chars<F>(self, f: F) -> Self
  where
    F: FnMut(char) -> char,
  {
    Self {
      inner: self.inner.chars().map(f).collect(),
    }
  }
}

impl Monad<char> for NonEmptyString {
  type HigherSelf<U: Send + Sync + 'static> = NonEmptyString;

  fn pure(a: char) -> Self::HigherSelf<char> {
    Self {
      inner: a.to_string(),
    }
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(char) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    let mut result = String::new();
    for c in self.inner.chars() {
      result.push_str(&f(c).inner);
    }
    Self { inner: result }
  }
}

impl FromStr for NonEmptyString {
  type Err = &'static str;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.is_empty() {
      Err("cannot create NonEmptyString from empty string")
    } else {
      Ok(Self {
        inner: s.to_string(),
      })
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_non_empty_string_creation() {
    let s = NonEmptyString::new("héllo".to_string()).unwrap();
    assert_eq!(s.len(), 6); // 'é' takes 2 bytes in UTF-8
    assert_eq!(s.first_char(), 'h');
    assert_eq!(s.as_str(), "héllo");
    assert!(!s.is_empty());
    assert!(!s.is_ascii());

    assert!(NonEmptyString::new(String::new()).is_none());
    assert!(NonEmptyString::try_from_str("").is_none());
  }

  #[test]
  fn test_from_str_trait() {
    let s = "hello".parse::<NonEmptyString>().unwrap();
    assert_eq!(s.as_str(), "hello");
    assert_eq!(s.len(), 5);
    assert!(!s.is_empty());

    let result = "".parse::<NonEmptyString>();
    assert!(result.is_err());
    assert_eq!(
      result.unwrap_err(),
      "cannot create NonEmptyString from empty string"
    );
  }

  #[test]
  fn test_non_empty_string_properties() {
    let s = NonEmptyString::new("héllo".to_string()).unwrap();
    assert_eq!(s.len(), 6); // 'é' takes 2 bytes in UTF-8
    assert!(!s.is_empty());
    assert!(!s.is_ascii());
    assert_eq!(s.first_char(), 'h');
    assert_eq!(s.as_str(), "héllo");

    let ascii = NonEmptyString::new("ASCII".to_string()).unwrap();
    assert!(ascii.is_ascii());

    let single_char = NonEmptyString::new("a".to_string()).unwrap();
    assert_eq!(single_char.len(), 1);
    assert!(!single_char.is_empty());
    assert!(single_char.is_ascii());
    assert_eq!(single_char.first_char(), 'a');
  }

  #[test]
  fn test_non_empty_string_conversion() {
    let original = "hello";
    let s = NonEmptyString::new(original.to_string()).unwrap();
    assert_eq!(s.into_string(), original);

    let s = NonEmptyString::new("hello".to_string()).unwrap();
    assert_eq!(s.as_str(), "hello");
  }

  #[test]
  fn test_non_empty_string_map_chars() {
    let s = NonEmptyString::new("hello".to_string()).unwrap();
    let mapped = s.map_chars(|c| c.to_ascii_uppercase());
    assert_eq!(mapped.as_str(), "HELLO");
    assert!(!mapped.is_empty());
    assert_eq!(mapped.len(), 5);

    let s = NonEmptyString::new("a".to_string()).unwrap();
    let mapped = s.map_chars(|c| c.to_ascii_uppercase());
    assert_eq!(mapped.as_str(), "A");
    assert!(!mapped.is_empty());
    assert_eq!(mapped.len(), 1);
  }

  #[test]
  fn test_monad_laws() {
    // Left identity
    let a = 'a';
    let f = |c: char| NonEmptyString::new(c.to_string().repeat(2)).unwrap();
    assert_eq!(NonEmptyString::pure(a).bind::<char, _>(f), f(a));

    // Right identity
    let m = NonEmptyString::new("hello".to_string()).unwrap();
    let m_clone = m.clone();
    assert_eq!(m.bind::<char, _>(NonEmptyString::pure), m_clone);

    // Associativity
    let m = NonEmptyString::new("hello".to_string()).unwrap();
    let m_clone = m.clone();
    let left = m.bind::<char, _>(move |c| {
      let f = |c: char| NonEmptyString::new(c.to_string().repeat(2)).unwrap();
      let g = |c: char| NonEmptyString::new(c.to_string().repeat(3)).unwrap();
      f(c).bind::<char, _>(g)
    });
    let right = m_clone
      .bind::<char, _>(|c| NonEmptyString::new(c.to_string().repeat(2)).unwrap())
      .bind::<char, _>(|c| NonEmptyString::new(c.to_string().repeat(3)).unwrap());
    assert_eq!(left, right);
  }

  #[test]
  fn test_edge_cases() {
    // Test with a single character
    let single = NonEmptyString::new("a".to_string()).unwrap();
    assert_eq!(single.len(), 1);
    assert!(!single.is_empty());
    assert!(single.is_ascii());
    assert_eq!(single.first_char(), 'a');
    assert_eq!(single.as_str(), "a");

    // Test with non-ASCII characters
    let non_ascii = NonEmptyString::new("こんにちは".to_string()).unwrap();
    assert_eq!(non_ascii.len(), 15); // Each character is 3 bytes in UTF-8
    assert!(!non_ascii.is_empty());
    assert!(!non_ascii.is_ascii());
    assert_eq!(non_ascii.first_char(), 'こ');

    // Test with whitespace
    let whitespace = NonEmptyString::new(" hello ".to_string()).unwrap();
    assert_eq!(whitespace.len(), 7);
    assert!(!whitespace.is_empty());
    assert!(whitespace.is_ascii());
    assert_eq!(whitespace.first_char(), ' ');
  }
}
