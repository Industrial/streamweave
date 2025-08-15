use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::VecDeque;

// Implement Functor directly for VecDeque<T>
impl<T: CloneableThreadSafe> Functor<T> for VecDeque<T> {
  type HigherSelf<U: CloneableThreadSafe> = VecDeque<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.into_iter().map(|x| f(&x)).collect()
  }

  fn map_owned<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized,
  {
    self.into_iter().map(|x| f(x)).collect()
  }
}

// Extension trait to make VecDeque mapping more ergonomic
pub trait VecDequeFunctorExt<T: CloneableThreadSafe> {
  fn map<U, F>(self, f: F) -> VecDeque<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;
}

// Implement the extension trait for VecDeque
impl<T: CloneableThreadSafe> VecDequeFunctorExt<T> for VecDeque<T> {
  fn map<U, F>(self, mut f: F) -> VecDeque<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.into_iter().map(|x| f(&x)).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper function to convert a Vec to a VecDeque
  fn to_vecdeque<T: Clone>(v: Vec<T>) -> VecDeque<T> {
    v.into_iter().collect()
  }

  // Define test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i32) -> i32] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i32::MAX),
  ];

  #[test]
  fn test_identity_law_empty() {
    let vecdeque: VecDeque<i32> = VecDeque::new();
    let id = |x: &i32| *x;
    let result = <VecDeque<i32> as Functor<i32>>::map(vecdeque, id);

    // Identity law for empty vector
    assert_eq!(result, VecDeque::<i32>::new());
  }

  #[test]
  fn test_identity_law_nonempty() {
    let xs = to_vecdeque(vec![1, 2, 3]);
    let vecdeque = xs.clone();
    let id = |x: &i32| *x;
    let result = <VecDeque<i32> as Functor<i32>>::map(vecdeque, id);

    // Identity law: functor.map(id) == functor
    assert_eq!(result, xs);
  }

  proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_identity_law_prop(
      xs in prop::collection::vec(any::<i32>(), 0..5)
    ) {
      let vecdeque = to_vecdeque(xs.clone());
      let id = |x: &i32| *x;
      let result = <VecDeque<i32> as Functor<i32>>::map(vecdeque, id);

      // Identity law: functor.map(id) == functor
      prop_assert_eq!(result, to_vecdeque(xs));
    }

    #[test]
    fn test_composition_law_empty(
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let vecdeque: VecDeque<i32> = VecDeque::new();
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Apply f then g
      let result1 = <VecDeque<i32> as Functor<i32>>::map(
        <VecDeque<i32> as Functor<i32>>::map(vecdeque.clone(), f),
        g
      );

      // Apply g ∘ f directly
      let result2 = <VecDeque<i32> as Functor<i32>>::map(vecdeque, move |x| g(&f(x)));

      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_composition_law_nonempty(
      xs in prop::collection::vec(
        any::<i32>(),
        1..5
      ),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let vecdeque = to_vecdeque(xs);
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Apply f then g
      let result1 = <VecDeque<i32> as Functor<i32>>::map(
        <VecDeque<i32> as Functor<i32>>::map(vecdeque.clone(), f),
        g
      );

      // Apply g ∘ f directly
      let result2 = <VecDeque<i32> as Functor<i32>>::map(vecdeque, move |x| g(&f(x)));

      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_map_with_simple_function(
      xs in prop::collection::vec(
        any::<i32>(),
        0..5
      )
    ) {
      let vecdeque = to_vecdeque(xs.clone());
      let f = |x: &i32| x.saturating_add(1);
      let result = <VecDeque<i32> as Functor<i32>>::map(vecdeque, f);

      let expected: Vec<i32> = xs.iter().map(|x| x.saturating_add(1)).collect();
      prop_assert_eq!(result, to_vecdeque(expected));
    }

    #[test]
    fn test_map_preserves_length(
      xs in prop::collection::vec(
        any::<i32>(),
        0..5
      )
    ) {
      let vecdeque = to_vecdeque(xs.clone());
      let f = |x: &i32| x.saturating_mul(2);
      let result = <VecDeque<i32> as Functor<i32>>::map(vecdeque, f);

      prop_assert_eq!(result.len(), xs.len());
    }

    #[test]
    fn test_map_with_complex_function(
      xs in prop::collection::vec(
        any::<i32>(),
        0..5
      )
    ) {
      let vecdeque = to_vecdeque(xs.clone());
      let f = |x: &i32| {
        let doubled = x.saturating_mul(2);
        if doubled > 0 { doubled.saturating_add(1) } else { doubled.saturating_sub(1) }
      };
      let result = <VecDeque<i32> as Functor<i32>>::map(vecdeque, f);

      let expected: Vec<i32> = xs.iter().map(|x| {
        let doubled = x.saturating_mul(2);
        if doubled > 0 { doubled.saturating_add(1) } else { doubled.saturating_sub(1) }
      }).collect();
      prop_assert_eq!(result, to_vecdeque(expected));
    }
  }
}
