use crate::functor::Functor;
use crate::monad::Monad;
use std::fmt::{Debug, Display};

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
  pub fn from_str(s: &str) -> Option<Self> {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_non_empty_string_creation() {
    let s = NonEmptyString::new("hello".to_string()).unwrap();
    assert_eq!(s.len(), 5);
    assert_eq!(s.first_char(), 'h');
    assert_eq!(s.as_str(), "hello");

    assert!(NonEmptyString::new(String::new()).is_none());
    assert!(NonEmptyString::from_str("").is_none());
  }

  #[test]
  fn test_non_empty_string_map_chars() {
    let s = NonEmptyString::new("hello".to_string()).unwrap();
    let mapped = s.map_chars(|c| c.to_ascii_uppercase());
    assert_eq!(mapped.as_str(), "HELLO");
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
}
