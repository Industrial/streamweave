use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::rc::Rc;

impl<T: CloneableThreadSafe> Foldable<T> for Rc<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    // For Rc, fold over the referenced value
    f(init, &self)
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // For Rc, fold_right over the referenced value
    f(&self, init)
  }

  fn reduce<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Rc, we need to clone the inner value
    Some((*self).clone())
  }

  fn reduce_right<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Rc, we need to clone the inner value
    Some((*self).clone())
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

  // Test fold with Rc
  #[test]
  fn test_fold() {
    let rc = Rc::new(5);
    let result = rc.fold(10, |acc, x| acc + x);
    assert_eq!(result, 15);
  }

  // Test fold_right with Rc
  #[test]
  fn test_fold_right() {
    let rc = Rc::new(5);
    let result = rc.fold_right(10, |x, acc| x + acc);
    assert_eq!(result, 15);
  }

  // Test reduce with Rc
  #[test]
  fn test_reduce() {
    let rc = Rc::new(42);
    let result = rc.reduce(|_, _| unreachable!());
    assert_eq!(result, Some(42));
  }

  // Test reduce_right with Rc
  #[test]
  fn test_reduce_right() {
    let rc = Rc::new(42);
    let result = rc.reduce_right(|_, _| unreachable!());
    assert_eq!(result, Some(42));
  }

  // Property-based test for fold
  proptest! {
    #[test]
    fn test_fold_properties(
      x in -1000..1000i32,
      init in -10..10i32,
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let rc = Rc::new(x);
      let f = INT_FUNCTIONS[f_idx];

      let result = rc.fold(init, f);
      let expected = f(init, &x);

      prop_assert_eq!(result, expected);
    }
  }

  // Property-based test for fold_right
  proptest! {
    #[test]
    fn test_fold_right_properties(
      x in -1000..1000i32,
      init in -10..10i32,
      f_idx in 0..INT_FUNCTIONS_RIGHT.len()
    ) {
      let rc = Rc::new(x);
      let f = INT_FUNCTIONS_RIGHT[f_idx];

      let result = rc.fold_right(init, f);
      let expected = f(&x, init);

      prop_assert_eq!(result, expected);
    }
  }

  // Property-based test for reduce (which simply clones the value)
  proptest! {
    #[test]
    fn test_reduce_properties(x in -1000..1000i32) {
      let rc = Rc::new(x);
      let result = rc.reduce(|_, _| unreachable!());

      prop_assert_eq!(result, Some(x));
    }
  }

  // Property-based test for reduce_right (which also simply clones the value)
  proptest! {
    #[test]
    fn test_reduce_right_properties(x in -1000..1000i32) {
      let rc = Rc::new(x);
      let result = rc.reduce_right(|_, _| unreachable!());

      prop_assert_eq!(result, Some(x));
    }
  }

  // Test with different types
  #[test]
  fn test_foldable_different_types() {
    // Rc with string value
    let rc = Rc::new("hello");
    let string_length = rc.fold(0, |acc, s| acc + s.len());
    assert_eq!(string_length, 5);

    // Rc with string value, used as an accumulator
    let rc = Rc::new(5);
    let as_string = rc.fold("Value: ".to_string(), |mut acc, x| {
      acc.push_str(&x.to_string());
      acc
    });
    assert_eq!(as_string, "Value: 5");

    // Rc with vector
    let rc = Rc::new(vec![1, 2, 3]);
    let sum = rc.fold(0, |acc, v| acc + v.iter().sum::<i32>());
    assert_eq!(sum, 6);
  }

  // Test with stateful function
  #[test]
  fn test_fold_with_stateful_function() {
    let rc = Rc::new(5);
    let mut call_count = 0;

    let result = rc.fold(10, |acc, x| {
      call_count += 1;
      acc + x
    });

    assert_eq!(result, 15);
    assert_eq!(call_count, 1, "Function should be called once");
  }

  // Test fold with complex values
  #[test]
  fn test_fold_with_complex_types() {
    // Rc with nested Option as its value
    let rc = Rc::new(Some(42));
    let result = rc.fold(0, |acc, opt| match opt {
      Some(x) => acc + x,
      None => acc,
    });
    assert_eq!(result, 42);

    // Fold that returns an Option
    let rc = Rc::new(5);
    let result: Option<i32> = rc.fold(Some(10), |acc, x| acc.map(|a| a + x));
    assert_eq!(result, Some(15));
  }

  // Test thread safety (compile-time check)
  #[test]
  fn test_foldable_thread_safety() {
    let rc = Rc::new(42);
    let result = rc.fold(10, |acc, x| acc + x);

    // This checks that the result is Send + Sync
    let handle = std::thread::spawn(move || {
      assert_eq!(result, 52);
    });

    handle.join().unwrap();
  }

  // Test for Foldable laws
  #[test]
  fn test_foldable_identity_law() {
    // Identity law: fold(foldable, init, |acc, _| acc) == init
    let rc = Rc::new(42);
    let init = 10;
    let result = rc.fold(init, |acc, _| acc);
    assert_eq!(result, init);
  }

  // Test with custom types
  #[test]
  fn test_foldable_with_custom_types() {
    // Define a custom type
    #[derive(Debug, Clone, PartialEq)]
    struct Counter {
      value: i32,
    }

    impl CloneableThreadSafe for Counter {}

    let rc = Rc::new(Counter { value: 5 });

    // Fold to increment the counter
    let result = rc.fold(Counter { value: 10 }, |mut acc, counter| {
      acc.value += counter.value;
      acc
    });

    assert_eq!(result, Counter { value: 15 });
  }

  // Test with Rc containing Rc
  #[test]
  fn test_foldable_with_nested_rc() {
    let inner_rc = Rc::new(5);
    let outer_rc = Rc::new(inner_rc);

    // Fold on the outer Rc gives us the inner Rc
    let result = outer_rc.fold(0, |acc, inner| acc + **inner);
    assert_eq!(result, 5);
  }
}
