//! Local (non-thread-safe) Monoid implementations.
//!
//! This module contains Monoid-like functionality for types that
//! cannot implement the Monoid trait due to thread-safety requirements.

use crate::local::semigroup::LocalSemigroup;
use std::rc::Rc;

/// A trait for non-thread-safe types that have Monoid-like behavior.
///
/// This is a local version of the Monoid trait for types that cannot
/// implement Send + Sync (such as Rc<T>).
pub trait LocalMonoid: LocalSemigroup {
  /// Returns the identity element of the monoid.
  fn local_empty() -> Self;

  /// Folds a collection of monoid values using the monoid operation.
  fn local_mconcat<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = Self>,
    Self: Sized,
  {
    iter
      .into_iter()
      .fold(Self::local_empty(), Self::local_combine)
  }
}

/// Implementation of LocalMonoid for Rc<i32>.
impl LocalMonoid for Rc<i32> {
  fn local_empty() -> Self {
    Rc::new(0)
  }
}

/// Implementation of LocalMonoid for Rc<String>.
impl LocalMonoid for Rc<String> {
  fn local_empty() -> Self {
    Rc::new(String::new())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    let empty_i32 = Rc::<i32>::local_empty();
    assert_eq!(*empty_i32, 0);

    let empty_string = Rc::<String>::local_empty();
    assert_eq!(*empty_string, "");
  }

  #[test]
  fn test_identity() {
    // Test with i32
    let a = Rc::new(10);
    let empty = Rc::<i32>::local_empty();

    // Left identity: empty.local_combine(a) = a
    assert_eq!(*empty.clone().local_combine(a.clone()), *a);

    // Right identity: a.local_combine(empty) = a
    assert_eq!(*a.clone().local_combine(empty), *a);

    // Test with String
    let s = Rc::new(String::from("test"));
    let empty = Rc::<String>::local_empty();

    // Left identity: empty.local_combine(s) = s
    assert_eq!(empty.clone().local_combine(s.clone()).as_ref(), s.as_ref());

    // Right identity: s.local_combine(empty) = s
    assert_eq!(s.clone().local_combine(empty).as_ref(), s.as_ref());
  }

  #[test]
  fn test_mconcat() {
    // Test with empty iterator
    let empty_vec: Vec<Rc<i32>> = Vec::new();
    let result = Rc::<i32>::local_mconcat(empty_vec);
    assert_eq!(*result, 0);

    // Test with non-empty iterator of i32
    let values = vec![Rc::new(1), Rc::new(2), Rc::new(3)];
    let result = Rc::<i32>::local_mconcat(values);
    assert_eq!(*result, 6); // 1 + 2 + 3 = 6

    // Test with non-empty iterator of String
    let strings = vec![
      Rc::new(String::from("Hello, ")),
      Rc::new(String::from("world")),
      Rc::new(String::from("!")),
    ];
    let result = Rc::<String>::local_mconcat(strings);
    assert_eq!(result.as_ref(), "Hello, world!");
  }

  proptest! {
    #[test]
    fn prop_left_identity_i32(a in any::<i32>()) {
      let rc_a = Rc::new(a);
      let empty = Rc::<i32>::local_empty();
      prop_assert_eq!(*empty.local_combine(rc_a.clone()), *rc_a);
    }

    #[test]
    fn prop_right_identity_i32(a in any::<i32>()) {
      let rc_a = Rc::new(a);
      let empty = Rc::<i32>::local_empty();
      prop_assert_eq!(*rc_a.clone().local_combine(empty), *rc_a);
    }

    #[test]
    fn prop_left_identity_string(a in ".*") {
      let rc_a = Rc::new(a.clone());
      let empty = Rc::<String>::local_empty();
      let combined = empty.local_combine(rc_a.clone());
      prop_assert_eq!(combined.as_ref(), rc_a.as_ref());
    }

    #[test]
    fn prop_right_identity_string(a in ".*") {
      let rc_a = Rc::new(a.clone());
      let empty = Rc::<String>::local_empty();
      let combined = rc_a.clone().local_combine(empty);
      prop_assert_eq!(combined.as_ref(), rc_a.as_ref());
    }

    #[test]
    fn prop_mconcat_is_fold_with_combine(items in prop::collection::vec(any::<i32>(), 0..10)) {
      let rc_items: Vec<Rc<i32>> = items.iter().map(|i| Rc::new(*i)).collect();

      let folded = rc_items.iter().cloned().fold(Rc::<i32>::local_empty(), |acc, item| acc.local_combine(item));
      let mconcated = Rc::<i32>::local_mconcat(rc_items.clone());

      prop_assert_eq!(*folded, *mconcated);
    }
  }
}
