//! Scannable trait and implementations.
//!
//! A scannable is a type that can perform cumulative operations on its elements,
//! producing intermediate results.

/// The Scannable trait defines operations for performing cumulative operations on elements.
pub trait Scannable<T: Send + Sync + 'static> {
  type HigherSelf<U>
  where
    U: Send + Sync + 'static;

  /// Performs a left scan (fold) operation, producing all intermediate results
  fn scan_left<B, F>(self, init: B, f: F) -> Self::HigherSelf<B>
  where
    B: Send + Sync + Clone + 'static,
    F: FnMut(B, T) -> B;

  /// Performs a right scan (fold) operation, producing all intermediate results
  fn scan_right<B, F>(self, init: B, f: F) -> Self::HigherSelf<B>
  where
    B: Send + Sync + Clone + 'static,
    F: FnMut(T, B) -> B;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Scannable<T> for Option<T> {
  type HigherSelf<U>
    = Option<U>
  where
    U: Send + Sync + 'static;

  fn scan_left<B, F>(self, init: B, mut f: F) -> Option<B>
  where
    B: Send + Sync + Clone + 'static,
    F: FnMut(B, T) -> B,
  {
    self.map(|x| f(init.clone(), x))
  }

  fn scan_right<B, F>(self, init: B, mut f: F) -> Option<B>
  where
    B: Send + Sync + Clone + 'static,
    F: FnMut(T, B) -> B,
  {
    self.map(|x| f(x, init.clone()))
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Scannable<T> for Vec<T> {
  type HigherSelf<U>
    = Vec<U>
  where
    U: Send + Sync + 'static;

  fn scan_left<B, F>(self, init: B, mut f: F) -> Vec<B>
  where
    B: Send + Sync + Clone + 'static,
    F: FnMut(B, T) -> B,
  {
    let mut result = Vec::with_capacity(self.len() + 1);
    result.push(init.clone());
    let mut acc = init;
    for x in self {
      acc = f(acc, x);
      result.push(acc.clone());
    }
    result
  }

  fn scan_right<B, F>(self, init: B, mut f: F) -> Vec<B>
  where
    B: Send + Sync + Clone + 'static,
    F: FnMut(T, B) -> B,
  {
    let mut result = Vec::with_capacity(self.len() + 1);
    let mut acc = init;
    result.push(acc.clone());
    for x in self.into_iter().rev() {
      acc = f(x, acc);
      result.push(acc.clone());
    }
    result.reverse();
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_option_scan_left() {
    let a = Some(5);
    let result = a.scan_left(1, |acc, x| acc * x);
    assert_eq!(result, Some(5));

    let a: Option<i32> = None;
    let result = a.scan_left(1, |acc, x| acc * x);
    assert_eq!(result, None);
  }

  #[test]
  fn test_option_scan_right() {
    let a = Some(5);
    let result = a.scan_right(1, |x, acc| x * acc);
    assert_eq!(result, Some(5));

    let a: Option<i32> = None;
    let result = a.scan_right(1, |x, acc| x * acc);
    assert_eq!(result, None);
  }

  #[test]
  fn test_vec_scan_left() {
    let a = vec![1, 2, 3, 4];
    let result = a.scan_left(0, |acc, x| acc + x);
    assert_eq!(result, vec![0, 1, 3, 6, 10]);

    let a: Vec<i32> = vec![];
    let result = a.scan_left(0, |acc, x| acc + x);
    assert_eq!(result, vec![0]);
  }

  #[test]
  fn test_vec_scan_right() {
    let a = vec![1, 2, 3, 4];
    let result = a.scan_right(0, |x, acc| x + acc);
    assert_eq!(result, vec![10, 9, 7, 4, 0]);

    let a: Vec<i32> = vec![];
    let result = a.scan_right(0, |x, acc| x + acc);
    assert_eq!(result, vec![0]);
  }

  #[test]
  fn test_type_conversions() {
    let a = vec![1, 2, 3];
    let result = a.scan_left(String::new(), |acc, x| format!("{}{}", acc, x));
    assert_eq!(result, vec!["", "1", "12", "123"]);

    let a = vec![1, 2, 3];
    let result = a.scan_right(String::new(), |x, acc| format!("{}{}", x, acc));
    assert_eq!(result, vec!["123", "23", "3", ""]);
  }
}
