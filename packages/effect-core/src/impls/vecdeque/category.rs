use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

/// A morphism for VecDeque<T> that represents transformations from one type to another
#[derive(Clone)]
pub struct VecDequeFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> VecDequeFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    VecDequeFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

/// A proxy struct to implement Category for VecDeque
#[derive(Clone, Copy)]
pub struct VecDequeCategory;

impl<T, U> Category<T, U> for VecDequeCategory
where
  T: CloneableThreadSafe,
  U: CloneableThreadSafe,
{
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = VecDequeFn<A, B>;

  /// The identity morphism for VecDeque - returns the input unchanged
  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    VecDequeFn::new(|x| x)
  }

  /// Compose two morphisms f: A -> B and g: B -> C to get a morphism A -> C
  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    VecDequeFn::new(move |x| g.apply(f.apply(x)))
  }

  /// Lift a regular function to a morphism
  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    VecDequeFn::new(move |x| f(&x))
  }

  /// Create a morphism that applies f to the first component of a pair
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    VecDequeFn::new(move |(a, c)| (f.apply(a), c))
  }

  /// Create a morphism that applies f to the second component of a pair
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    VecDequeFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::collections::VecDeque;

  /// Helper function to convert a Vec to a VecDeque
  fn to_vecdeque<T: Clone>(v: Vec<T>) -> VecDeque<T> {
    v.into_iter().collect()
  }

  /// Helper function to reverse a VecDeque
  fn reverse<T: Clone>(deque: &VecDeque<T>) -> VecDeque<T> {
    let mut result = VecDeque::with_capacity(deque.len());
    for item in deque.iter().rev() {
      result.push_back(item.clone());
    }
    result
  }

  /// Helper function to filter elements in a VecDeque
  fn filter_even(deque: &VecDeque<i32>) -> VecDeque<i32> {
    deque.iter().filter(|&x| x % 2 == 0).cloned().collect()
  }

  /// Helper function to map elements in a VecDeque
  fn double_elements(deque: &VecDeque<i32>) -> VecDeque<i32> {
    deque.iter().map(|x| x * 2).collect()
  }

  #[test]
  fn test_identity_law() {
    // Create a test VecDeque
    let deque = to_vecdeque(vec![1, 2, 3]);

    // Create the identity morphism
    let id = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::id();

    // Apply it to the deque
    let result = id.apply(deque.clone());

    // Identity should preserve the original deque
    assert_eq!(result, deque);
  }

  #[test]
  fn test_composition_law() {
    // Create a test VecDeque
    let deque = to_vecdeque(vec![1, 2, 3, 4]);

    // Create two morphisms: filter even numbers, then double them
    let f = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(filter_even);
    let g = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(double_elements);

    // Compose them
    let f_then_g =
      <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::compose(f.clone(), g.clone());

    // Apply the composed function
    let result = f_then_g.apply(deque.clone());

    // Manually verify the expected result
    let filtered = filter_even(&deque);
    let expected = double_elements(&filtered);

    assert_eq!(result, expected);

    // The result should be [4, 8] (even numbers doubled)
    let expected_manual = to_vecdeque(vec![4, 8]);
    assert_eq!(result, expected_manual);
  }

  #[test]
  fn test_first() {
    // Create a test VecDeque
    let deque = to_vecdeque(vec![1, 2]);

    // Create a value to pair with
    let value = "test".to_string();

    // Create a morphism and its "first" version
    let f = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(double_elements);
    let first_f = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::first(f);

    // Apply the first morphism to a pair
    let result = first_f.apply((deque.clone(), value.clone()));

    // Verify the result
    assert_eq!(result.0, double_elements(&deque));
    assert_eq!(result.1, value);
  }

  #[test]
  fn test_second() {
    // Create a test VecDeque
    let deque = to_vecdeque(vec![1, 2]);

    // Create a value to pair with
    let value = "test".to_string();

    // Create a morphism and its "second" version
    let f = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(double_elements);
    let second_f = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::second(f);

    // Apply the second morphism to a pair
    let result = second_f.apply((value.clone(), deque.clone()));

    // Verify the result
    assert_eq!(result.0, value);
    assert_eq!(result.1, double_elements(&deque));
  }

  #[test]
  fn test_arr() {
    // Create a test VecDeque
    let deque = to_vecdeque(vec![1, 2, 3]);

    // Create a morphism using arr
    let double_morphism =
      <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(double_elements);

    // Apply the morphism
    let result = double_morphism.apply(deque.clone());

    // Verify the result
    assert_eq!(result, double_elements(&deque));
  }

  #[test]
  fn test_category_laws() {
    // Create a test VecDeque
    let deque = to_vecdeque(vec![1, 2, 3]);

    // Create morphisms
    let f = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(filter_even);
    let g = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(double_elements);
    let h = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(reverse);

    // Test associativity: (f . g) . h = f . (g . h)
    let fg =
      <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::compose(f.clone(), g.clone());
    let fg_h = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::compose(fg, h.clone());

    let gh = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::compose(g, h);
    let f_gh = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::compose(f, gh);

    // Apply both compositions
    let result1 = fg_h.apply(deque.clone());
    let result2 = f_gh.apply(deque.clone());

    // The results should be the same
    assert_eq!(result1, result2);
  }

  #[test]
  fn test_with_different_types() {
    // Create a test VecDeque with strings
    let deque = to_vecdeque(vec!["hello".to_string(), "world".to_string()]);

    // Create a function that transforms strings
    fn to_uppercase(deque: &VecDeque<String>) -> VecDeque<String> {
      deque.iter().map(|s| s.to_uppercase()).collect()
    }

    // Create a morphism using arr
    let uppercase_morphism =
      <VecDequeCategory as Category<VecDeque<String>, VecDeque<String>>>::arr(to_uppercase);

    // Apply the morphism
    let result = uppercase_morphism.apply(deque.clone());

    // Expected result
    let expected = to_vecdeque(vec!["HELLO".to_string(), "WORLD".to_string()]);

    // Verify the result
    assert_eq!(result, expected);
  }

  proptest! {
    #[test]
    fn prop_identity_law(
      elements in prop::collection::vec(1..100i32, 0..10)
    ) {
      let deque: VecDeque<i32> = elements.clone().into_iter().collect();
      let id = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::id();
      let result = id.apply(deque.clone());
      prop_assert_eq!(result, deque);
    }

    #[test]
    fn prop_composition_preserves_structure(
      elements in prop::collection::vec(1..100i32, 0..10)
    ) {
      let deque: VecDeque<i32> = elements.clone().into_iter().collect();

      // Create morphisms
      let f = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(filter_even);
      let g = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::arr(double_elements);

      // Compose and apply
      let composed = <VecDequeCategory as Category<VecDeque<i32>, VecDeque<i32>>>::compose(f.clone(), g.clone());
      let composed_result = composed.apply(deque.clone());

      // Apply individually
      let f_result = f.apply(deque.clone());
      let expected = g.apply(f_result);

      prop_assert_eq!(composed_result, expected);
    }
  }
}
