//! Functor trait and implementations.
//!
//! A functor is a type that can be mapped over, preserving structure.
//!
//!

use std::collections::HashMap;

/// Marker trait for types that can be mapped over.
pub trait Mappable {}

// Implement Mappable for all types
impl<T> Mappable for T {}

/// The Functor trait defines the basic operations for functor types.
pub trait Functor<A> {
  /// The type constructor for the functor.
  type HigherSelf<T>: Functor<T> + Mappable;

  /// Maps a function over the functor.
  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B;
}

// Implementation for Option
impl<A> Functor<A> for Option<A> {
  type HigherSelf<T> = Option<T>;

  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B,
  {
    self.map(f)
  }
}

// Implementation for Vec
impl<A> Functor<A> for Vec<A> {
  type HigherSelf<T> = Vec<T>;

  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B,
  {
    self.into_iter().map(f).collect()
  }
}

// Implementation for Result
impl<A, E> Functor<A> for Result<A, E> {
  type HigherSelf<T> = Result<T, E>;

  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B,
  {
    self.map(f)
  }
}

// Implementation for Box
impl<A> Functor<A> for Box<A> {
  type HigherSelf<T> = Box<T>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B,
  {
    Box::new(f(*self))
  }
}

// Implementation for HashMap (values only)
impl<K: std::hash::Hash + Eq, V> Functor<V> for HashMap<K, V> {
  type HigherSelf<T> = HashMap<K, T>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(V) -> B,
  {
    self.into_iter().map(|(k, v)| (k, f(v))).collect()
  }
}

// Either type
#[derive(Debug, PartialEq)]
pub enum Either<L, R> {
  Left(L),
  Right(R),
}

// Implementation for Either
impl<A, R> Functor<A> for Either<A, R> {
  type HigherSelf<T> = Either<T, R>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B,
  {
    match self {
      Either::Left(v) => Either::Left(f(v)),
      Either::Right(r) => Either::Right(r),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod option_tests {
    #[test]
    fn test_some_to_some() {
      let some = Some(42);
      let mapped = some.map(|x| x * 2);
      assert_eq!(mapped, Some(84));
    }

    #[test]
    fn test_none_to_none() {
      let none: Option<i32> = None;
      let mapped = none.map(|x| x * 2);
      assert_eq!(mapped, None);
    }

    #[test]
    fn test_type_conversion() {
      let some = Some(42);
      let mapped = some.map(|x| x.to_string());
      assert_eq!(mapped, Some("42".to_string()));
    }

    #[test]
    fn test_identity() {
      let some = Some(42);
      let mapped = some.map(|x| x);
      assert_eq!(mapped, some);
    }

    #[test]
    fn test_composition() {
      let some = Some(42);
      let mapped = some.map(|x| x * 2).map(|x| x + 1);
      assert_eq!(mapped, Some(85));
    }
  }

  mod vec_tests {
    use super::*;

    #[test]
    fn test_empty_vec() {
      let empty: Vec<i32> = vec![];
      let mapped = empty.map(|x| x * 2);
      let expected: Vec<i32> = vec![];
      assert_eq!(mapped, expected);
    }

    #[test]
    fn test_single_element() {
      let vec = vec![42];
      let mapped = vec.map(|x| x * 2);
      assert_eq!(mapped, vec![84]);
    }

    #[test]
    fn test_multiple_elements() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x * 2);
      assert_eq!(mapped, vec![2, 4, 6]);
    }

    #[test]
    fn test_type_conversion() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x.to_string());
      assert_eq!(mapped, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_identity() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x);
      assert_eq!(mapped, vec![1, 2, 3]);
    }

    #[test]
    fn test_composition() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x * 2).map(|x| x + 1);
      assert_eq!(mapped, vec![3, 5, 7]);
    }
  }

  mod result_tests {
    #[test]
    fn test_ok_to_ok() {
      let ok: Result<i32, &str> = Ok(42);
      let mapped = ok.map(|x| x * 2);
      assert_eq!(mapped, Ok(84));
    }

    #[test]
    fn test_err_to_err() {
      let err: Result<i32, &str> = Err("error");
      let mapped = err.map(|x| x * 2);
      assert_eq!(mapped, Err("error"));
    }

    #[test]
    fn test_type_conversion() {
      let ok: Result<i32, &str> = Ok(42);
      let mapped = ok.map(|x| x.to_string());
      assert_eq!(mapped, Ok("42".to_string()));
    }
  }

  mod box_tests {
    use super::*;

    #[test]
    fn test_box_map() {
      let bx = Box::new(42);
      let mapped = bx.map(|x| x * 2);
      assert_eq!(*mapped, 84);
    }

    #[test]
    fn test_type_conversion() {
      let bx = Box::new(42);
      let mapped = bx.map(|x| x.to_string());
      assert_eq!(*mapped, "42".to_string());
    }
  }

  mod hashmap_tests {
    use super::*;

    #[test]
    fn test_hashmap_map() {
      let mut map = HashMap::new();
      map.insert("one", 1);
      map.insert("two", 2);
      let mapped = map.map(|x| x * 2);
      assert_eq!(mapped.get("one"), Some(&2));
      assert_eq!(mapped.get("two"), Some(&4));
    }

    #[test]
    fn test_empty_hashmap() {
      let map: HashMap<&str, i32> = HashMap::new();
      let mapped = map.map(|x| x * 2);
      assert!(mapped.is_empty());
    }
  }
}
