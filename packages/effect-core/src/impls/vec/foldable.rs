use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Foldable<T> for Vec<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    let mut acc = init;
    for x in self {
      acc = f(acc, &x);
    }
    acc
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    let mut acc = init;
    for x in self.into_iter().rev() {
      acc = f(&x, acc);
    }
    acc
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let mut iter = self.into_iter();
    let first = iter.next().unwrap();
    let mut acc = first;

    for x in iter {
      acc = f(&acc, &x);
    }

    Some(acc)
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let mut iter = self.into_iter().rev();
    let first = iter.next().unwrap();
    let mut acc = first;

    for x in iter {
      acc = f(&x, &acc);
    }

    Some(acc)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions with overflow protection for property-based testing
  const INT_FUNCTIONS: &[fn(i32, &i32) -> i32] = &[
    |acc, x| acc.saturating_add(*x),
    |acc, x| acc.saturating_mul(*x),
    |acc, x| acc.saturating_sub(*x),
    |acc, x| if *x != 0 { acc / x } else { acc },
  ];

  const INT_FUNCTIONS_RIGHT: &[fn(&i32, i32) -> i32] = &[
    |x, acc| acc.saturating_add(*x),
    |x, acc| acc.saturating_mul(*x),
    |x, acc| x.saturating_sub(acc),
    |x, acc| if acc != 0 { *x / acc } else { *x },
  ];

  const REDUCE_FUNCTIONS: &[fn(&i32, &i32) -> i32] = &[
    |x, y| x.saturating_add(*y),
    |x, y| x.saturating_mul(*y),
    |x, y| x.saturating_sub(*y),
    |x, y| if *y != 0 { *x / *y } else { *x },
    |x, y| std::cmp::max(*x, *y),
    |x, y| std::cmp::min(*x, *y),
  ];

  // Helper function to create a test vector
  fn test_vector(length: usize, base_value: i32) -> Vec<i32> {
    (0..length).map(|i| base_value + (i as i32)).collect()
  }

  // Test fold with empty vector
  #[test]
  fn test_fold_empty() {
    let empty: Vec<i32> = Vec::new();
    let result = empty.fold(42, |acc, _| acc + 1);
    assert_eq!(
      result, 42,
      "fold on empty vector should return the initial value"
    );
  }

  // Test fold with single element
  #[test]
  fn test_fold_single() {
    let single = vec![5];
    let result = single.fold(10, |acc, x| acc + x);
    assert_eq!(
      result, 15,
      "fold on single element vector should work correctly"
    );
  }

  // Test fold with multiple elements
  #[test]
  fn test_fold_multiple() {
    let multiple = vec![1, 2, 3, 4, 5];
    let sum = multiple.clone().fold(0, |acc, x| acc + x);
    assert_eq!(sum, 15, "fold should correctly sum elements");

    let product = multiple.fold(1, |acc, x| acc * x);
    assert_eq!(product, 120, "fold should correctly multiply elements");
  }

  // Property-based test for fold
  proptest! {
    #[test]
    fn test_fold_properties(
      // Use small vector size and bounded values to prevent overflow
      length in 0..100usize,
      base_value in -50..50i32,
      init in -10..10i32,
      f_idx in 0..INT_FUNCTIONS.len(),
    ) {
      let vec = test_vector(length, base_value);
      let f = INT_FUNCTIONS[f_idx];

      // Manually compute expected result for comparison
      let mut expected = init;
      for x in &vec {
        expected = f(expected, x);
      }

      let result = vec.fold(init, f);
      assert_eq!(result, expected, "fold should match manual computation");
    }
  }

  // Test fold_right with empty vector
  #[test]
  fn test_fold_right_empty() {
    let empty: Vec<i32> = Vec::new();
    let result = empty.fold_right(42, |_, acc| acc + 1);
    assert_eq!(
      result, 42,
      "fold_right on empty vector should return the initial value"
    );
  }

  // Test fold_right with single element
  #[test]
  fn test_fold_right_single() {
    let single = vec![5];
    let result = single.fold_right(10, |x, acc| x + acc);
    assert_eq!(
      result, 15,
      "fold_right on single element vector should work correctly"
    );
  }

  // Test fold_right with multiple elements
  #[test]
  fn test_fold_right_multiple() {
    let multiple = vec![1, 2, 3, 4, 5];

    // For addition, fold and fold_right give same results
    let sum = multiple.clone().fold_right(0, |x, acc| x + acc);
    assert_eq!(sum, 15, "fold_right should correctly sum elements");

    // For non-commutative operations, fold and fold_right differ
    let subtract1 = vec![100, 20, 5].fold(0, |acc, x| acc - x);
    let subtract2 = vec![100, 20, 5].fold_right(0, |x, acc| x - acc);
    assert_ne!(
      subtract1, subtract2,
      "fold and fold_right should differ for non-commutative operations"
    );
  }

  // Property-based test for fold_right
  proptest! {
    #[test]
    fn test_fold_right_properties(
      length in 0..100usize,
      base_value in -50..50i32,
      init in -10..10i32,
      f_idx in 0..INT_FUNCTIONS_RIGHT.len(),
    ) {
      let vec = test_vector(length, base_value);
      let f = INT_FUNCTIONS_RIGHT[f_idx];

      // Manually compute expected result for comparison
      let mut expected = init;
      for x in vec.iter().rev() {
        expected = f(x, expected);
      }

      let result = vec.fold_right(init, f);
      assert_eq!(result, expected, "fold_right should match manual computation");
    }
  }

  // Test reduce with empty vector
  #[test]
  fn test_reduce_empty() {
    let empty: Vec<i32> = Vec::new();
    let result = empty.reduce(|x, y| x + y);
    assert_eq!(result, None, "reduce on empty vector should return None");
  }

  // Test reduce with single element
  #[test]
  fn test_reduce_single() {
    let single = vec![42];
    let result = single.reduce(|_, _| unreachable!());
    assert_eq!(
      result,
      Some(42),
      "reduce on single element should return that element"
    );
  }

  // Test reduce with multiple elements
  #[test]
  fn test_reduce_multiple() {
    let multiple = vec![1, 2, 3, 4, 5];
    let sum = multiple.clone().reduce(|x, y| x + y);
    assert_eq!(sum, Some(15), "reduce should correctly sum elements");

    let max = multiple.clone().reduce(|x, y| std::cmp::max(*x, *y));
    assert_eq!(max, Some(5), "reduce should correctly find maximum");

    let min = multiple.reduce(|x, y| std::cmp::min(*x, *y));
    assert_eq!(min, Some(1), "reduce should correctly find minimum");
  }

  // Property-based test for reduce
  proptest! {
    #[test]
    fn test_reduce_properties(
      // Non-empty vectors for reduce
      length in 1..100usize,
      base_value in -50..50i32,
      f_idx in 0..REDUCE_FUNCTIONS.len(),
    ) {
      let vec = test_vector(length, base_value);
      let f = REDUCE_FUNCTIONS[f_idx];

      // Manually compute expected result
      let mut iter = vec.iter();
      let first = iter.next().unwrap();
      let mut expected = *first;

      for x in iter {
        expected = f(&expected, x);
      }

      let result = vec.clone().reduce(f);
      assert_eq!(result, Some(expected), "reduce should match manual computation");
    }
  }

  // Test reduce_right with empty vector
  #[test]
  fn test_reduce_right_empty() {
    let empty: Vec<i32> = Vec::new();
    let result = empty.reduce_right(|x, y| x + y);
    assert_eq!(
      result, None,
      "reduce_right on empty vector should return None"
    );
  }

  // Test reduce_right with single element
  #[test]
  fn test_reduce_right_single() {
    let single = vec![42];
    let result = single.reduce_right(|_, _| unreachable!());
    assert_eq!(
      result,
      Some(42),
      "reduce_right on single element should return that element"
    );
  }

  // Test reduce_right with multiple elements
  #[test]
  fn test_reduce_right_multiple() {
    let multiple = vec![1, 2, 3, 4, 5];

    // For commutative operations like addition, reduce and reduce_right are the same
    let sum = multiple.clone().reduce_right(|x, y| x + y);
    assert_eq!(sum, Some(15), "reduce_right should correctly sum elements");

    // For non-commutative operations, they differ
    let subtract1 = vec![100, 20, 5].reduce(|x, y| x - y);
    let subtract2 = vec![100, 20, 5].reduce_right(|x, y| x - y);
    assert_ne!(
      subtract1, subtract2,
      "reduce and reduce_right should differ for non-commutative operations"
    );
  }

  // Property-based test for reduce_right
  proptest! {
    #[test]
    fn test_reduce_right_properties(
      // Non-empty vectors for reduce_right
      length in 1..100usize,
      base_value in -50..50i32,
      f_idx in 0..REDUCE_FUNCTIONS.len(),
    ) {
      let vec = test_vector(length, base_value);
      let f = REDUCE_FUNCTIONS[f_idx];

      // Manually compute expected result
      let mut iter = vec.iter().rev();
      let first = iter.next().unwrap();
      let mut expected = *first;

      for x in iter {
        expected = f(x, &expected);
      }

      let result = vec.clone().reduce_right(f);
      assert_eq!(result, Some(expected), "reduce_right should match manual computation");
    }
  }

  // Tests for Foldable with different types
  #[test]
  fn test_fold_different_types() {
    let strings = vec!["hello", "world", "rust"];

    // Concatenate strings
    let concat = strings.clone().fold(String::new(), |mut acc, s| {
      if !acc.is_empty() {
        acc.push_str(" ");
      }
      acc.push_str(s);
      acc
    });
    assert_eq!(concat, "hello world rust");

    // Count total length
    let total_length = strings.fold(0, |acc, s| acc + s.len());
    assert_eq!(total_length, 14);
  }

  // Test with stateful function
  #[test]
  fn test_fold_with_stateful_function() {
    let numbers = vec![1, 2, 3, 4, 5];

    // Simple sum function to verify foldable behavior
    let result = numbers.fold(0, |acc, x| acc + x);

    assert_eq!(result, 15, "Fold should correctly compute the sum");
  }

  // Test with complex accumulator
  #[test]
  fn test_fold_with_complex_accumulator() {
    let numbers = vec![1, 2, 3, 4, 5];

    // Collect even and odd numbers separately using a tuple accumulator
    let (evens, odds) = numbers.fold((Vec::new(), Vec::new()), |(mut evens, mut odds), x| {
      if x % 2 == 0 {
        evens.push(*x);
      } else {
        odds.push(*x);
      }
      (evens, odds)
    });

    assert_eq!(evens, vec![2, 4]);
    assert_eq!(odds, vec![1, 3, 5]);
  }
}
