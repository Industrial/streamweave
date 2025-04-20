use crate::traits::monoid::Monoid;
use crate::traits::semigroup::Semigroup;
use std::sync::Arc;

impl<T> Monoid for Arc<T>
where
  T: Monoid + Clone,
{
  fn empty() -> Self {
    // Delegate to the inner type's empty implementation
    Arc::new(T::empty())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    let empty_i32: Arc<i32> = Monoid::empty();
    assert_eq!(*empty_i32, 0);

    let empty_string: Arc<String> = Monoid::empty();
    assert_eq!(*empty_string, String::new());
  }

  #[test]
  fn test_identity() {
    // Test identity with i32
    let a = Arc::new(10);
    let empty: Arc<i32> = Monoid::empty();

    // Left identity: empty.combine(a) = a
    assert_eq!(*empty.clone().combine(a.clone()), *a);

    // Right identity: a.combine(empty) = a
    assert_eq!(*a.clone().combine(empty), *a);

    // Test identity with String
    let s = Arc::new(String::from("test"));
    let empty: Arc<String> = Monoid::empty();

    // Left identity: empty.combine(s) = s
    assert_eq!(empty.clone().combine(s.clone()).as_ref(), s.as_ref());

    // Right identity: s.combine(empty) = s
    assert_eq!(s.clone().combine(empty).as_ref(), s.as_ref());
  }

  #[test]
  fn test_mconcat() {
    // Test with empty iterator
    let empty_vec: Vec<Arc<i32>> = Vec::new();
    let result = Monoid::mconcat(empty_vec);
    assert_eq!(*result, 0);

    // Test with non-empty iterator of i32
    let values = vec![Arc::new(1), Arc::new(2), Arc::new(3)];
    let result = Monoid::mconcat(values);
    assert_eq!(*result, 6); // 1 + 2 + 3 = 6

    // Test with non-empty iterator of String
    let strings = vec![
      Arc::new(String::from("Hello, ")),
      Arc::new(String::from("world")),
      Arc::new(String::from("!")),
    ];
    let result = Monoid::mconcat(strings);
    assert_eq!(result.as_ref(), "Hello, world!");
  }

  proptest! {
    #[test]
    fn prop_left_identity_i32(a in any::<i32>()) {
      let arc_a = Arc::new(a);
      let empty: Arc<i32> = Monoid::empty();
      prop_assert_eq!(*empty.combine(arc_a.clone()), *arc_a);
    }

    #[test]
    fn prop_right_identity_i32(a in any::<i32>()) {
      let arc_a = Arc::new(a);
      let empty: Arc<i32> = Monoid::empty();
      prop_assert_eq!(*arc_a.clone().combine(empty), *arc_a);
    }

    #[test]
    fn prop_left_identity_string(a in ".*") {
      let arc_a = Arc::new(a.clone());
      let empty: Arc<String> = Monoid::empty();
      let combined = empty.combine(arc_a.clone());
      prop_assert_eq!(combined.as_ref(), arc_a.as_ref());
    }

    #[test]
    fn prop_right_identity_string(a in ".*") {
      let arc_a = Arc::new(a.clone());
      let empty: Arc<String> = Monoid::empty();
      let combined = arc_a.clone().combine(empty);
      prop_assert_eq!(combined.as_ref(), arc_a.as_ref());
    }

    #[test]
    fn prop_mconcat_is_fold_with_combine(items in prop::collection::vec(any::<i32>(), 0..10)) {
      let arc_items: Vec<Arc<i32>> = items.iter().map(|i| Arc::new(*i)).collect();

      let folded = arc_items.iter().cloned().fold(Monoid::empty(), |acc: Arc<i32>, item| acc.combine(item));
      let mconcated = Monoid::mconcat(arc_items.clone());

      prop_assert_eq!(*folded, *mconcated);
    }
  }
}
