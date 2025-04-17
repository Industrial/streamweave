use std::option::Option;
use std::time::Duration;
use std::vec::Vec;

/// A trait for types that can throttle or debounce their elements.
pub trait Throttleable<T> {
  /// Throttles elements by emitting at most one element per specified duration.
  /// For collections, this simulates throttling by keeping elements that would be emitted
  /// based on the duration and count.
  fn throttle(self, duration: Duration, count: usize) -> Self;

  /// Debounces elements by emitting only after no new elements arrive within the specified duration.
  /// For collections, this simulates debouncing by keeping only elements that would survive the debounce.
  fn debounce(self, duration: Duration) -> Self;
}

impl<T: Clone> Throttleable<T> for Vec<T> {
  fn throttle(self, _duration: Duration, count: usize) -> Vec<T> {
    if count == 0 {
      return Vec::new();
    }
    // Simulate throttling by taking every nth element
    self
      .into_iter()
      .enumerate()
      .filter(|(i, _)| i % count == 0)
      .map(|(_, v)| v)
      .collect()
  }

  fn debounce(self, _duration: Duration) -> Vec<T> {
    if self.is_empty() {
      return Vec::new();
    }
    // Simulate debouncing by keeping only the last element
    vec![self.into_iter().last().unwrap()]
  }
}

impl<T: Clone> Throttleable<T> for Option<T> {
  fn throttle(self, _duration: Duration, count: usize) -> Option<T> {
    if count == 0 {
      None
    } else {
      self
    }
  }

  fn debounce(self, _duration: Duration) -> Option<T> {
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_throttle() {
    let v = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(
      v.clone().throttle(Duration::from_secs(1), 2),
      vec![1, 3, 5, 7, 9]
    );
    assert_eq!(
      v.clone().throttle(Duration::from_secs(1), 3),
      vec![1, 4, 7, 10]
    );
    assert_eq!(v.clone().throttle(Duration::from_secs(1), 1), v);
    assert!(v.clone().throttle(Duration::from_secs(1), 0).is_empty());
  }

  #[test]
  fn test_vec_debounce() {
    let v = vec![1, 2, 3, 4, 5];
    assert_eq!(v.clone().debounce(Duration::from_secs(1)), vec![5]);

    let empty: Vec<i32> = vec![];
    assert!(empty.debounce(Duration::from_secs(1)).is_empty());
  }

  #[test]
  fn test_option_throttle() {
    let o: Option<i32> = Some(42);
    assert_eq!(o.clone().throttle(Duration::from_secs(1), 1), Some(42));
    assert_eq!(o.clone().throttle(Duration::from_secs(1), 2), Some(42));
    assert_eq!(o.throttle(Duration::from_secs(1), 0), None);

    let none: Option<i32> = None;
    assert_eq!(none.throttle(Duration::from_secs(1), 1), None);
  }

  #[test]
  fn test_option_debounce() {
    let o: Option<i32> = Some(42);
    assert_eq!(o.debounce(Duration::from_secs(1)), Some(42));

    let none: Option<i32> = None;
    assert_eq!(none.debounce(Duration::from_secs(1)), None);
  }
}
