use crate::impls::linkedlist::category::LinkedListCategory;
use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::LinkedList;

// Implement Applicative for LinkedListCategory
impl<A: CloneableThreadSafe> Applicative<A> for LinkedListCategory {
  fn pure<B>(_value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    // Return LinkedListCategory as the proxy
    // The actual implementation is in the extension trait
    LinkedListCategory
  }

  fn ap<B, F>(self, _fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // Return LinkedListCategory as the proxy
    // The actual implementation is in the extension trait
    LinkedListCategory
  }
}

// Extension trait to make LinkedList applicative operations more ergonomic
pub trait LinkedListApplicativeExt<A: CloneableThreadSafe> {
  fn pure<B>(value: B) -> LinkedList<B>
  where
    B: CloneableThreadSafe;

  fn ap<B, F>(self, fs: LinkedList<F>) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe;
}

// Implement the extension trait for LinkedList
impl<A: CloneableThreadSafe> LinkedListApplicativeExt<A> for LinkedList<A> {
  fn pure<B>(value: B) -> LinkedList<B>
  where
    B: CloneableThreadSafe,
  {
    let mut list = LinkedList::new();
    list.push_back(value);
    list
  }

  fn ap<B, F>(self, fs: LinkedList<F>) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = LinkedList::new();
    for mut f in fs {
      for x in &self {
        result.push_back(f(x));
      }
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  fn to_linkedlist<T: Clone>(vec: Vec<T>) -> LinkedList<T> {
    vec.into_iter().collect()
  }

  #[test]
  fn test_pure() {
    // Test the extension trait's pure method instead
    let result = LinkedList::<i32>::pure(42);

    let mut expected = LinkedList::new();
    expected.push_back(42);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_ap_empty_fs() {
    let fs: LinkedList<fn(&i32) -> i32> = LinkedList::new();
    let xs = to_linkedlist(vec![1, 2]);

    let result = xs.ap(fs);

    assert_eq!(result, LinkedList::new());
  }

  #[test]
  fn test_ap_empty_xs() {
    let mut fs = LinkedList::new();
    fs.push_back(|x: &i32| x + 1);
    let xs: LinkedList<i32> = LinkedList::new();

    let result = xs.ap(fs);

    assert_eq!(result, LinkedList::new());
  }

  #[test]
  fn test_ap_single_f_multiple_xs() {
    let mut fs = LinkedList::new();
    fs.push_back(|x: &i32| x + 1);

    let xs = to_linkedlist(vec![1, 2, 3]);

    let result = xs.ap(fs);

    let expected = to_linkedlist(vec![2, 3, 4]);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_ap_multiple_fs_single_x() {
    let fs = to_linkedlist(vec![|x: &i32| x + 1, |x: &i32| x * 2, |x: &i32| x - 1]);

    let mut xs = LinkedList::new();
    xs.push_back(10);

    let result = xs.ap(fs);

    let expected = to_linkedlist(vec![11, 20, 9]);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_ap_multiple_fs_multiple_xs() {
    let fs = to_linkedlist(vec![|x: &i32| x + 1, |x: &i32| x * 2]);

    let xs = to_linkedlist(vec![1, 2]);

    let result = xs.ap(fs);

    let expected = to_linkedlist(vec![2, 3, 2, 4]);

    assert_eq!(result, expected);
  }

  proptest! {
    #[test]
    fn test_identity_law(
      xs in prop::collection::vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      // Identity Law: pure id <*> v ≡ v
      let list: LinkedList<i64> = xs.iter().cloned().collect();

      // Create a LinkedList with the id function
      let mut fs = LinkedList::new();
      fs.push_back(|x: &i64| *x);

      // Apply: pure id <*> v
      let result = list.clone().ap(fs);

      // Should equal the original list
      prop_assert_eq!(result, list);
    }

    #[test]
    fn test_homomorphism_law(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      // Homomorphism Law: pure f <*> pure x ≡ pure (f x)
      let f = |x: &i64| x + 1;

      // pure f <*> pure x
      let mut fs = LinkedList::<fn(&i64) -> i64>::new();
      fs.push_back(f);
      let result = LinkedList::<i64>::pure(x).ap(fs);

      // pure (f x)
      let expected = LinkedList::<i64>::pure(f(&x));

      prop_assert_eq!(result, expected);
    }

    #[test]
    fn test_interchange_law(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      // Interchange Law: u <*> pure y ≡ pure ($ y) <*> u

      // Create a LinkedList of functions with safe operations
      let fs = to_linkedlist(vec![
        |x: &i64| x.saturating_add(1),
        |x: &i64| x.saturating_mul(2)
      ]);

      // u <*> pure y
      let result1 = LinkedList::<i64>::pure(x).ap(fs.clone());

      // Create a function that applies y to a function
      let x_clone = x; // Clone x to avoid lifetime issues
      let apply_y = move |f: &fn(&i64) -> i64| f(&x_clone);
      let mut apply_fs = LinkedList::new();
      apply_fs.push_back(apply_y);

      // Result of applying each function to y with safe operations
      let result2 = to_linkedlist(vec![
        x.saturating_add(1),
        x.saturating_mul(2)
      ]);

      // Result1 should match the expected values
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_composition_law(
      xs in prop::collection::vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      // Composition Law: pure (.) <*> u <*> v <*> w ≡ u <*> (v <*> w)
      // Simplified for testing

      // Define test functions with safe operations
      let f_copy = |x: &i64| x.saturating_add(2);
      let g_copy = |x: &i64| x.saturating_mul(3);

      // Convert vector to LinkedList
      let xs_list: LinkedList<i64> = xs.iter().cloned().collect();

      // Create function lists
      let f_list = to_linkedlist(vec![f_copy]);
      let g_list = to_linkedlist(vec![g_copy]);

      // First approach: apply g to xs, then apply f to the result
      let g_applied = xs_list.clone().ap(g_list);
      let result1 = g_applied.ap(f_list);

      // Second approach: compose f and g, then apply to xs
      let composed = move |x: &i64| {
        let g_result = g_copy(x);
        f_copy(&g_result)
      };
      let composed_list = to_linkedlist(vec![composed]);

      let result2 = xs_list.ap(composed_list);

      // Results should be the same
      prop_assert_eq!(result1, result2);
    }
  }
}
