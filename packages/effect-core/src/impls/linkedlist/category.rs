use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

/// A morphism for LinkedList<T> that represents transformations from one type to another
#[derive(Clone)]
pub struct LinkedListFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> LinkedListFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    LinkedListFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

/// A proxy struct to implement Category for LinkedList
pub struct LinkedListCategory;

impl<T, U> Category<T, U> for LinkedListCategory
where
  T: CloneableThreadSafe,
  U: CloneableThreadSafe,
{
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = LinkedListFn<A, B>;

  /// The identity morphism for LinkedList - returns the input unchanged
  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    LinkedListFn::new(|x| x)
  }

  /// Compose two morphisms f: A -> B and g: B -> C to get a morphism A -> C
  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    LinkedListFn::new(move |x| g.apply(f.apply(x)))
  }

  /// Lift a regular function to a morphism
  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    LinkedListFn::new(move |x| f(&x))
  }

  /// Create a morphism that applies f to the first component of a pair
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    LinkedListFn::new(move |(a, c)| (f.apply(a), c))
  }

  /// Create a morphism that applies f to the second component of a pair
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    LinkedListFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::collections::LinkedList;

  /// Helper function to reverse a LinkedList
  fn reverse<T: Clone>(list: &LinkedList<T>) -> LinkedList<T> {
    let mut result = LinkedList::new();
    for item in list.iter() {
      result.push_front(item.clone());
    }
    result
  }

  /// Helper function to filter elements in a LinkedList
  fn filter_even(list: &LinkedList<i32>) -> LinkedList<i32> {
    let mut result = LinkedList::new();
    for item in list.iter() {
      if item % 2 == 0 {
        result.push_back(*item);
      }
    }
    result
  }

  /// Helper function to map elements in a LinkedList
  fn double_elements(list: &LinkedList<i32>) -> LinkedList<i32> {
    let mut result = LinkedList::new();
    for item in list.iter() {
      result.push_back(item * 2);
    }
    result
  }

  #[test]
  fn test_identity_law() {
    // Create a test LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);

    // Create the identity morphism
    let id = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::id();

    // Apply it to the list
    let result = id.apply(list.clone());

    // Identity should preserve the original list
    assert_eq!(result, list);
  }

  #[test]
  fn test_composition_law() {
    // Create a test LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);
    list.push_back(4);

    // Create two morphisms: filter even numbers, then double them
    let f = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(filter_even);
    let g =
      <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(double_elements);

    // Compose them
    let f_then_g = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::compose(
      f.clone(),
      g.clone(),
    );

    // Apply the composed function
    let result = f_then_g.apply(list.clone());

    // Manually verify the expected result
    let filtered = filter_even(&list);
    let expected = double_elements(&filtered);

    assert_eq!(result, expected);

    // The result should be [4, 8] (even numbers doubled)
    let mut manually_created = LinkedList::new();
    manually_created.push_back(4);
    manually_created.push_back(8);
    assert_eq!(result, manually_created);
  }

  #[test]
  fn test_first() {
    // Create a test LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);

    // Create a value to pair with
    let value = "test".to_string();

    // Create a morphism and its "first" version
    let f =
      <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(double_elements);
    let first_f = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::first(f);

    // Apply the first morphism to a pair
    let result = first_f.apply((list.clone(), value.clone()));

    // Verify the result
    assert_eq!(result.0, double_elements(&list));
    assert_eq!(result.1, value);
  }

  #[test]
  fn test_second() {
    // Create a test LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);

    // Create a value to pair with
    let value = "test".to_string();

    // Create a morphism and its "second" version
    let f =
      <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(double_elements);
    let second_f = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::second(f);

    // Apply the second morphism to a pair
    let result = second_f.apply((value.clone(), list.clone()));

    // Verify the result
    assert_eq!(result.0, value);
    assert_eq!(result.1, double_elements(&list));
  }

  #[test]
  fn test_arr() {
    // Create a test LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);

    // Create a morphism using arr
    let double_morphism =
      <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(double_elements);

    // Apply the morphism
    let result = double_morphism.apply(list.clone());

    // Verify the result
    assert_eq!(result, double_elements(&list));
  }

  #[test]
  fn test_category_laws() {
    // Create a test LinkedList
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);

    // Create morphisms
    let f = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(filter_even);
    let g =
      <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(double_elements);
    let h = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(reverse);

    // Test associativity: (f . g) . h = f . (g . h)
    let fg = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::compose(
      f.clone(),
      g.clone(),
    );
    let fg_h =
      <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::compose(fg, h.clone());

    let gh = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::compose(g, h);
    let f_gh = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::compose(f, gh);

    // Apply both compositions
    let result1 = fg_h.apply(list.clone());
    let result2 = f_gh.apply(list.clone());

    // The results should be the same
    assert_eq!(result1, result2);
  }

  proptest! {
    #[test]
    fn prop_identity_law(
      elements in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      let id = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::id();
      let result = id.apply(elements.clone());
      prop_assert_eq!(result, elements);
    }

    #[test]
    fn prop_composition_preserves_structure(
      elements in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      // Create morphisms
      let f = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(filter_even);
      let g = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::arr(double_elements);

      // Compose and apply
      let composed = <LinkedListCategory as Category<LinkedList<i32>, LinkedList<i32>>>::compose(f.clone(), g.clone());
      let composed_result = composed.apply(elements.clone());

      // Apply individually
      let f_result = f.apply(elements.clone());
      let expected = g.apply(f_result);

      prop_assert_eq!(composed_result, expected);
    }
  }
}
