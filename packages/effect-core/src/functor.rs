//! Functor trait and implementations.
//!
//! A functor is a type that can be mapped over, preserving structure.
//!
//!

use std::collections::HashMap;
use std::hash::Hash;

/// The Functor trait defines the basic operations for functor types.
pub trait Functor<T> {
  /// The higher-kinded type that results from mapping over this functor
  type HigherSelf<U: Send + Sync + 'static>: Functor<U>;

  /// Maps a function over the functor
  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static;
}

/// A trait for types that can be mapped over
pub trait Mappable<T>: Functor<T> {}

// Implementation for Option
impl<T: Send + Sync + 'static> Functor<T> for Option<T> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self.map(|x| f(x))
  }
}

impl<T: Send + Sync + 'static> Mappable<T> for Option<T> {}

// Implementation for Vec
impl<T: Send + Sync + 'static> Functor<T> for Vec<T> {
  type HigherSelf<U: Send + Sync + 'static> = Vec<U>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self.into_iter().map(|x| f(x)).collect()
  }
}

impl<T: Send + Sync + 'static> Mappable<T> for Vec<T> {}

// Implementation for Result
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Functor<T> for Result<T, E> {
  type HigherSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self.map(|x| f(x))
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Mappable<T> for Result<T, E> {}

// Implementation for Box
impl<T: Send + Sync + 'static> Functor<T> for Box<T> {
  type HigherSelf<U: Send + Sync + 'static> = Box<U>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    Box::new(f(*self))
  }
}

// Implementation for HashMap
impl<K: Send + Sync + 'static + Eq + Hash, V: Send + Sync + 'static> Functor<V> for HashMap<K, V> {
  type HigherSelf<U: Send + Sync + 'static> = HashMap<K, U>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(V) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self.into_iter().map(|(k, v)| (k, f(v))).collect()
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
