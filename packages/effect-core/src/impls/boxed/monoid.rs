use crate::traits::monoid::Monoid;

impl<T> Monoid for Box<T>
where
  T: Monoid,
{
  fn empty() -> Self {
    // Delegate to the inner type's empty implementation
    Box::new(T::empty())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    let empty_i32: Box<i32> = Monoid::empty();
    assert_eq!(*empty_i32, 0);

    let empty_string: Box<String> = Monoid::empty();
    assert_eq!(*empty_string, String::new());
  }

  #[test]
  fn test_left_identity() {
    let a = Box::new(10);
    let empty: Box<i32> = Monoid::empty();
    assert_eq!(*empty.combine(a.clone()), *a);

    let s = Box::new(String::from("test"));
    let empty: Box<String> = Monoid::empty();
    assert_eq!(*empty.combine(s.clone()), *s);
  }

  #[test]
  fn test_right_identity() {
    let a = Box::new(10);
    let empty: Box<i32> = Monoid::empty();
    assert_eq!(*a.clone().combine(empty), *a);

    let s = Box::new(String::from("test"));
    let empty: Box<String> = Monoid::empty();
    assert_eq!(*s.clone().combine(empty), *s);
  }

  #[test]
  fn test_monoid_laws() {
    // Test with i32
    let a = Box::new(10);
    let empty: Box<i32> = Monoid::empty();

    // Left identity: empty.combine(a) = a
    assert_eq!(*empty.clone().combine(a.clone()), *a);

    // Right identity: a.combine(empty) = a
    assert_eq!(*a.clone().combine(empty), *a);

    // Test with String
    let s = Box::new(String::from("test"));
    let empty: Box<String> = Monoid::empty();

    // Left identity: empty.combine(s) = s
    assert_eq!(*empty.clone().combine(s.clone()), *s);

    // Right identity: s.combine(empty) = s
    assert_eq!(*s.clone().combine(empty), *s);
  }

  proptest! {
    #[test]
    fn prop_left_identity_i32(a in any::<i32>()) {
      let boxed_a = Box::new(a);
      let empty: Box<i32> = Monoid::empty();
      prop_assert_eq!(*empty.combine(boxed_a.clone()), *boxed_a);
    }

    #[test]
    fn prop_right_identity_i32(a in any::<i32>()) {
      let boxed_a = Box::new(a);
      let empty: Box<i32> = Monoid::empty();
      prop_assert_eq!(*boxed_a.clone().combine(empty), *boxed_a);
    }

    #[test]
    fn prop_left_identity_string(a in ".*") {
      let boxed_a = Box::new(a.clone());
      let empty: Box<String> = Monoid::empty();
      prop_assert_eq!(*empty.combine(boxed_a.clone()), *boxed_a);
    }

    #[test]
    fn prop_right_identity_string(a in ".*") {
      let boxed_a = Box::new(a.clone());
      let empty: Box<String> = Monoid::empty();
      prop_assert_eq!(*boxed_a.clone().combine(empty), *boxed_a);
    }

    #[test]
    fn prop_mconcat_is_fold_with_combine(items in prop::collection::vec(any::<i32>(), 0..10)) {
      let boxed_items: Vec<Box<i32>> = items.iter().map(|i| Box::new(*i)).collect();

      let folded = boxed_items.iter().cloned().fold(Monoid::empty(), |acc: Box<i32>, item| acc.combine(item));
      let mconcated = Monoid::mconcat(boxed_items.clone());

      prop_assert_eq!(*folded, *mconcated);
    }
  }
}
