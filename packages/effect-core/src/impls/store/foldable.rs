use crate::traits::foldable::Foldable;
use crate::types::store::Store;
use crate::types::threadsafe::CloneableThreadSafe;

impl<S: CloneableThreadSafe, A: CloneableThreadSafe> Foldable<A> for Store<S, A> {
  fn fold<B, F>(self, init: B, f: F) -> B
  where
    B: CloneableThreadSafe,
    F: for<'a> FnMut(B, &'a A) -> B + CloneableThreadSafe,
  {
    // For a Store, fold applies the function f to the current extract value
    // This is somewhat limited since we can only access the current position's value
    let value = self.extract();
    let mut f = f;
    f(init, &value)
  }

  fn fold_right<B, F>(self, init: B, f: F) -> B
  where
    B: CloneableThreadSafe,
    F: for<'a> FnMut(&'a A, B) -> B + CloneableThreadSafe,
  {
    // For right fold, since Store has only one focused value, this is essentially the same
    // as fold, but with the arguments to f reversed
    let value = self.extract();
    let mut f = f;
    f(&value, init)
  }

  fn reduce<F>(self, _f: F) -> Option<A>
  where
    F: for<'a, 'b> FnMut(&'a A, &'b A) -> A + CloneableThreadSafe,
  {
    // For Store, we only have a single accessible value at the current position,
    // so reduce doesn't make much sense. We return the extracted value wrapped in Some.
    Some(self.extract())
  }

  fn reduce_right<F>(self, _f: F) -> Option<A>
  where
    F: for<'a, 'b> FnMut(&'a A, &'b A) -> A + CloneableThreadSafe,
  {
    // Same as reduce for Store
    Some(self.extract())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_fold() {
    // Create a Store that doubles the position
    let store = Store::new(|x: &i32| x * 2, 5);

    // Fold with addition
    let result = store.fold(0, |acc, x| acc + x);

    // Result should be 0 + (5 * 2) = 10
    assert_eq!(result, 10);
  }

  #[test]
  fn test_fold_right() {
    // Create a Store that doubles the position
    let store = Store::new(|x: &i32| x * 2, 5);

    // Fold right with subtraction (order matters)
    let result = store.fold_right(0, |x, acc| *x - acc);

    // Result should be (5 * 2) - 0 = 10
    assert_eq!(result, 10);
  }

  #[test]
  fn test_reduce() {
    // Create a Store that doubles the position
    let store = Store::new(|x: &i32| x * 2, 5);

    // Reduce with addition (function won't actually be used)
    let result = store.reduce(|a, b| a + b);

    // Result should be Some(10) (the extract value)
    assert_eq!(result, Some(10));
  }

  // Property-based tests
  proptest! {
      #[test]
      fn prop_identity_law(init in any::<i32>(), pos in any::<i32>()) {
          // Identity law: fold(store, init, |acc, _| acc) == init
          // Use a safer function to avoid overflow
          let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);

          let result = store.fold(init, |acc, _| acc);

          prop_assert_eq!(result, init);
      }

      #[test]
      fn prop_fold_consistency(init in any::<i32>(), pos in any::<i32>()) {
          // Verify that fold applies the function correctly
          // Use a safer function to avoid overflow
          let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);

          // First store the extract value to use later
          let extract_value = store.extract();

          // Using addition as the fold operation with wrapping to avoid overflow
          let result = store.fold(init, |acc, x| acc.wrapping_add(*x));

          // The expected result is init + extract
          let expected = init.wrapping_add(extract_value);

          prop_assert_eq!(result, expected);
      }

      #[test]
      fn prop_fold_right_consistency(init in any::<i32>(), pos in any::<i32>()) {
          // Verify that fold_right applies the function correctly
          // Use a safer function to avoid overflow
          let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);

          // First store the extract value to use later
          let extract_value = store.extract();

          // Using subtraction as the fold operation with wrapping to avoid overflow
          let result = store.fold_right(init, |x, acc| x.wrapping_sub(acc));

          // The expected result is extract - init
          let expected = extract_value.wrapping_sub(init);

          prop_assert_eq!(result, expected);
      }
  }
}
