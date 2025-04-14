use crate::semigroup::Semigroup;

/// A trait for types that form a monoid under some operation.
/// A monoid is a type with an associative binary operation and an identity element.
pub trait Monoid: Semigroup {
  /// The identity element for the monoid operation.
  fn empty() -> Self;

  /// Fold a collection of monoid values using the monoid operation.
  fn mconcat<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = Self>,
  {
    iter.into_iter().fold(Self::empty(), Self::combine)
  }
}

// Implement Monoid for common types

impl<T: Clone> Monoid for Vec<T> {
  fn empty() -> Self {
    Vec::new()
  }
}

impl Monoid for String {
  fn empty() -> Self {
    String::new()
  }
}

// Implement Monoid for integer types
macro_rules! impl_integer_monoid {
  ($($t:ty),*) => {
    $(
      impl Monoid for $t {
        fn empty() -> Self {
          0
        }
      }
    )*
  };
}

impl_integer_monoid!(
  i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

// Implement Monoid for floating-point types
macro_rules! impl_float_monoid {
  ($($t:ty),*) => {
    $(
      impl Monoid for $t {
        fn empty() -> Self {
          0.0
        }
      }
    )*
  };
}

impl_float_monoid!(f32, f64);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_monoid() {
    let empty: Vec<i32> = Vec::empty();
    assert!(empty.is_empty());

    let v1 = vec![1, 2, 3];
    let v2 = vec![4, 5, 6];
    let combined = v1.combine(v2);
    assert_eq!(combined, vec![1, 2, 3, 4, 5, 6]);

    let concat = Vec::mconcat(vec![vec![1], vec![2], vec![3]]);
    assert_eq!(concat, vec![1, 2, 3]);
  }

  #[test]
  fn test_string_monoid() {
    let empty: String = String::empty();
    assert!(empty.is_empty());

    let s1 = "hello".to_string();
    let s2 = " world".to_string();
    let combined = s1.combine(s2);
    assert_eq!(combined, "hello world");

    let concat = String::mconcat(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert_eq!(concat, "abc");
  }

  #[test]
  fn test_integer_monoid() {
    let zero: i32 = i32::empty();
    assert_eq!(zero, 0);

    let sum = 1.combine(2);
    assert_eq!(sum, 3);

    let concat = i32::mconcat(vec![1, 2, 3]);
    assert_eq!(concat, 6);
  }

  #[test]
  fn test_float_monoid() {
    let zero: f32 = f32::empty();
    assert_eq!(zero, 0.0);

    let sum = 1.0_f32.combine(2.0);
    assert_eq!(sum, 3.0);

    let concat = f32::mconcat(vec![1.0, 2.0, 3.0]);
    assert_eq!(concat, 6.0);
  }
}
