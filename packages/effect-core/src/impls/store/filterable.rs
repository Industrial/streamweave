use crate::traits::filterable::Filterable;
use crate::types::store::Store;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;
use std::sync::Mutex;

impl<S: CloneableThreadSafe, A: CloneableThreadSafe> Filterable<A> for Store<S, A> {
  type Filtered<B: CloneableThreadSafe> = Store<S, Option<B>>;

  fn filter_map<B, F>(self, f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // For Store, filter_map returns a Store of Option<B>
    // If the function f returns None, the value at that position becomes None
    // Otherwise, it becomes Some(value)

    let original_peek = self.peek;

    // Wrap the mutable function f in a Mutex to make it thread-safe
    let f = Arc::new(Mutex::new(f));

    // The new peek function applies f to the result of the original peek
    // and returns an Option<B>
    let new_peek = Arc::new(move |s: &S| {
      let value = (original_peek)(s);
      let mut f_guard = f.lock().unwrap();
      (*f_guard)(&value)
    });

    Store {
      peek: new_peek,
      pos: self.pos,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::functor::Functor;
  use proptest::prelude::*;

  #[test]
  fn test_filter_map() {
    // Create a Store that doubles the position
    let store = Store::new(|x: &i32| x.wrapping_mul(2), 5);

    // Filter out even values
    let filter_fn = |x: &i32| {
      if x % 2 == 0 {
        Some(x.wrapping_add(1))
      } else {
        None
      }
    };

    // Apply filter_map
    let result = store.filter_map(filter_fn);

    // At position 5, value is 10, which is even, so result is Some(11)
    assert_eq!(result.extract(), Some(11));

    // At position 3, value is 6, which is even, so result is Some(7)
    assert_eq!(result.peek_at(&3), Some(7));

    // At position 4, value is 8, which is even, so result is Some(9)
    assert_eq!(result.peek_at(&4), Some(9));
  }

  #[test]
  fn test_filter() {
    // Create a Store that doubles the position
    let store = Store::new(|x: &i32| x.wrapping_mul(2), 5);

    // Filter to keep only values > 6
    let predicate = |x: &i32| *x > 6;

    // Apply filter
    let result = store.filter(predicate);

    // At position 5, value is 10, which is > 6, so result is Some(10)
    assert_eq!(result.extract(), Some(10));

    // At position 3, value is 6, which is not > 6, so result is None
    assert_eq!(result.peek_at(&3), None);

    // At position 4, value is 8, which is > 6, so result is Some(8)
    assert_eq!(result.peek_at(&4), Some(8));
  }

  // Property-based tests
  proptest! {
      #[test]
      fn prop_identity_preservation(pos in any::<i32>()) {
          // Identity preservation: store.filter_map(|x| Some(x)) == store
          let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);

          let result = store.clone().filter_map(|x| Some(x.clone()));

          // Check that all values are preserved in the Option wrapper
          prop_assert_eq!(result.extract(), Some(store.extract()));
          prop_assert_eq!(result.peek_at(&10), Some(store.peek_at(&10)));
      }

      #[test]
      fn prop_distributivity(pos in any::<i32>()) {
          // Distributivity: store.filter_map(f).filter_map(g) == store.filter_map(|x| f(x).and_then(g))
          let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);

          // Define two filter functions with wrapping operations
          let f = |x: &i32| if x % 2 == 0 { Some(x.wrapping_add(1)) } else { None };
          let g = |x: &i32| if *x > 5 { Some(x.wrapping_mul(2)) } else { None };

          // Left side: store.filter_map(f).filter_map(g)
          let left = store.clone()
              .filter_map(f)
              .filter_map(move |opt_x| opt_x.and_then(|x| g(&x)));

          // Right side: store.filter_map(|x| f(x).and_then(g))
          let right = store.filter_map(move |x| f(x).and_then(|y| g(&y)));

          // Compare results
          prop_assert_eq!(left.extract(), right.extract());
          prop_assert_eq!(left.peek_at(&10), right.peek_at(&10));
      }

      #[test]
      fn prop_consistency_with_functor(pos in any::<i32>()) {
          // Consistency with Functor: store.filter_map(|x| Some(f(x))) == store.map(f)
          let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);

          // Define a function for mapping
          fn add_one(x: &i32) -> i32 {
              x.wrapping_add(1)
          }

          // Left side: store.filter_map(|x| Some(f(x)))
          let left = store.clone().filter_map(|x| Some(add_one(x)));

          // Right side: store.map(f)
          let right = Functor::map(store, add_one);

          // Compare results, accounting for Option wrapper
          prop_assert_eq!(left.extract(), Some(right.extract()));
          prop_assert_eq!(left.peek_at(&10), Some(right.peek_at(&10)));
      }
  }
}
