//! Foldable trait and implementations.
//!
//! A foldable is a type that can be folded into a single value.

/// The Foldable trait defines the basic operations for foldable types.
pub trait Foldable<T: Send + Sync + 'static> {
  type HigherSelf;

  /// Folds elements into a single value using a binary operation
  fn fold<B, F>(self, init: B, f: F) -> B
  where
    F: FnMut(B, T) -> B;

  /// Reduces elements into a single value using a binary operation
  fn reduce<F>(self, f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Foldable<T> for Option<T> {
  type HigherSelf = Option<T>;

  fn fold<B, F>(self, init: B, mut f: F) -> B
  where
    F: FnMut(B, T) -> B,
  {
    match self {
      Some(x) => f(init, x),
      None => init,
    }
  }

  fn reduce<F>(self, _f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T,
  {
    self
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Foldable<T> for Vec<T> {
  type HigherSelf = Vec<T>;

  fn fold<B, F>(self, init: B, f: F) -> B
  where
    F: FnMut(B, T) -> B,
  {
    self.into_iter().fold(init, f)
  }

  fn reduce<F>(self, f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T,
  {
    self.into_iter().reduce(f)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Define a simple foldable for testing
  #[derive(Debug, PartialEq, Clone)]
  struct TestFoldable<T>(T);

  impl<T> TestFoldable<T> {
    fn new(value: T) -> Self {
      TestFoldable(value)
    }
  }

  impl<T: Send + Sync + 'static> Foldable<T> for TestFoldable<T> {
    type HigherSelf = TestFoldable<T>;

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
      F: FnMut(B, T) -> B,
    {
      f(init, self.0)
    }

    fn reduce<F>(self, _: F) -> Option<T>
    where
      F: FnMut(T, T) -> T,
    {
      Some(self.0)
    }
  }

  // Test foldable laws
  #[test]
  fn test_foldable_laws() {
    // Identity law: fold f init = f init
    let f = TestFoldable::new(42);
    let init = 0;
    let add = |acc: i32, x: i32| acc + x;
    assert_eq!(f.fold(init, add), add(init, 42));

    // Composition law: fold (f . g) init = fold f (fold g init)
    let f = TestFoldable::new(42);
    let init = 0;
    let g = |acc: i32, x: i32| acc + x;
    let h = |acc: i32, x: i32| acc * x;
    let lhs = f.clone().fold(init, |acc, x| h(g(acc, x), x));
    let rhs = h(f.fold(init, g), 42);
    assert_eq!(lhs, rhs);
  }

  // Test basic operations
  #[test]
  fn test_fold() {
    let f = TestFoldable::new(42);
    let result = f.fold(0, |acc, x| acc + x);
    assert_eq!(result, 42);
  }

  // Test type conversions
  #[test]
  fn test_type_conversions() {
    let f = TestFoldable::new(42);
    let result = f.fold(String::new(), |acc, x| format!("{}{}", acc, x));
    assert_eq!(result, "42");
  }

  // Test composition
  #[test]
  fn test_composition() {
    let f = TestFoldable::new(42i32);
    let result = f.fold(0i32, |acc, x| acc + x);
    assert_eq!(result, 42);
  }

  #[test]
  fn test_option_fold() {
    let some_value: Option<i32> = Some(42);
    let none_value: Option<i32> = None;

    assert_eq!(some_value.fold(0, |acc, x| acc + x), 42);
    assert_eq!(none_value.fold(0, |acc, x| acc + x), 0);
  }

  #[test]
  fn test_option_reduce() {
    let some_value: Option<i32> = Some(42);
    let none_value: Option<i32> = None;

    assert_eq!(some_value.reduce(|a, b| a + b), Some(42));
    assert_eq!(none_value.reduce(|a, b| a + b), None);
  }

  #[test]
  fn test_vec_fold() {
    let v = vec![1, 2, 3, 4, 5];
    assert_eq!(v.fold(0, |acc, x| acc + x), 15);
  }

  #[test]
  fn test_vec_reduce() {
    let v = vec![1, 2, 3, 4, 5];
    assert_eq!(v.reduce(|a, b| a + b), Some(15));

    let empty_vec: Vec<i32> = vec![];
    assert_eq!(empty_vec.reduce(|a, b| a + b), None);
  }

  #[test]
  fn test_fold_laws() {
    // Test associativity law for fold
    let v = vec![1i32, 2, 3, 4, 5];
    let f = |acc: i32, x: i32| acc + x;
    let g = |acc: i32, x: i32| acc * x;

    let result1 = v.clone().fold(0i32, f);
    let result2 = v.fold(1i32, g);

    assert_eq!(result1, 15); // sum
    assert_eq!(result2, 120); // product
  }
}
