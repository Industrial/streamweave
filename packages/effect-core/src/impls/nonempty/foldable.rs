// Foldable implementation for NonEmpty

use crate::traits::foldable::Foldable;
use crate::types::nonempty::NonEmpty;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Foldable<T> for NonEmpty<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    // Start with the initial value and fold over the head and tail
    let acc = f(init, &self.head);
    self.tail.iter().fold(acc, f)
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // Fold from the right (tail to head)
    let acc = self.tail.iter().rev().fold(init, |acc, x| f(x, acc));
    f(&self.head, acc)
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For NonEmpty, reduce always returns Some value
    // Start with the head and reduce over the tail
    Some(self.tail.iter().fold(self.head, |acc, x| f(&acc, x)))
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For NonEmpty, reduce_right always returns Some value
    if self.tail.is_empty() {
      return Some(self.head);
    }

    // Start from the rightmost element
    let mut acc = self.tail[self.tail.len() - 1].clone();

    // Process remaining tail elements from right to left
    for i in (0..self.tail.len() - 1).rev() {
      acc = f(&self.tail[i], &acc);
    }

    // Finally, combine with the head
    Some(f(&self.head, &acc))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_fold() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);

    // Sum all elements
    let sum = Foldable::fold(ne.clone(), 0, |acc, x| acc + x);
    assert_eq!(sum, 10);

    // Multiply all elements
    let product = Foldable::fold(ne, 1, |acc, x| acc * x);
    assert_eq!(product, 24);
  }

  #[test]
  fn test_fold_right() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);

    // String concat from right
    let result = Foldable::fold_right(ne, String::new(), |x, acc| format!("{}{}", x, acc));
    assert_eq!(result, "1234");
  }

  #[test]
  fn test_reduce() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);

    // Sum all elements
    let sum = Foldable::reduce(ne.clone(), |a, b| a + b);
    assert_eq!(sum, Some(10));

    // Max of all elements
    let max = Foldable::reduce(ne, |a, b| if a >= b { *a } else { *b });
    assert_eq!(max, Some(4));
  }

  #[test]
  fn test_reduce_right() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);

    // Sum from right (which gives the same result for addition)
    let sum = Foldable::reduce_right(ne, |a, b| a + b);
    assert_eq!(sum, Some(10));
  }

  #[test]
  fn test_reduce_right_string() {
    let ne = NonEmpty::from_parts(
      "1".to_string(),
      vec!["2".to_string(), "3".to_string(), "4".to_string()],
    );

    // String concat from right
    let result = Foldable::reduce_right(ne, |a, b| a.clone() + &b);
    assert_eq!(result, Some("1234".to_string()));
  }

  #[test]
  fn test_reduce_single_element() {
    let ne = NonEmpty::new(42);

    let result = Foldable::reduce(ne.clone(), |a, b| a + b);
    assert_eq!(result, Some(42));

    let result = Foldable::reduce_right(ne, |a, b| a + b);
    assert_eq!(result, Some(42));
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // Identity: fold(foldable, init, |acc, _| acc) == init
      let ne = NonEmpty::from_parts(head, tail);
      let init = 42;

      let result = Foldable::fold(ne, init, |acc, _| acc);
      prop_assert_eq!(result, init);
    }

    #[test]
    fn prop_fold_vs_fold_right(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // For commutative operations like addition, fold and fold_right should yield the same result
      let ne = NonEmpty::from_parts(head, tail);

      let sum_left = Foldable::fold(ne.clone(), 0, |acc: i32, x: &i32| acc.wrapping_add(*x));
      let sum_right = Foldable::fold_right(ne, 0, |x: &i32, acc: i32| acc.wrapping_add(*x));

      prop_assert_eq!(sum_left, sum_right);
    }

    #[test]
    fn prop_reduce_vs_fold(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // reduce with + should be the same as fold with + and 0 as initial value
      let ne = NonEmpty::from_parts(head, tail);

      let sum_fold = Foldable::fold(ne.clone(), 0, |acc: i32, x: &i32| acc.wrapping_add(*x));
      let sum_reduce = Foldable::reduce(ne, |a: &i32, b: &i32| a.wrapping_add(*b)).unwrap_or(0);

      prop_assert_eq!(sum_fold, sum_reduce); // No need to add 0
    }
  }
}
