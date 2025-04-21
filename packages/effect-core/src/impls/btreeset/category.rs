use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

/// A morphism for BTreeSet<T> that represents transformations from one set to another
#[derive(Clone)]
pub struct BTreeSetFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> BTreeSetFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    BTreeSetFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

/// A proxy struct to implement Category for BTreeSet
pub struct BTreeSetCategory;

impl<T, U> Category<T, U> for BTreeSetCategory
where
  T: CloneableThreadSafe,
  U: CloneableThreadSafe,
{
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = BTreeSetFn<A, B>;

  /// The identity morphism for BTreeSet - returns the input unchanged
  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    BTreeSetFn::new(|x| x)
  }

  /// Compose two morphisms f: A -> B and g: B -> C to get a morphism A -> C
  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    BTreeSetFn::new(move |x| g.apply(f.apply(x)))
  }

  /// Lift a regular function to a morphism
  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    BTreeSetFn::new(move |x| f(&x))
  }

  /// Create a morphism that applies f to the first component of a pair
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    BTreeSetFn::new(move |(a, c)| (f.apply(a), c))
  }

  /// Create a morphism that applies f to the second component of a pair
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    BTreeSetFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::collections::BTreeSet;
  use std::iter::FromIterator;

  /// Helper function to create a BTreeSet from a Vec
  fn to_btreeset<T: Ord + Clone>(v: Vec<T>) -> BTreeSet<T> {
    BTreeSet::from_iter(v)
  }

  /// Helper function to get the union of two sets
  fn union<T: Ord + Clone>(set1: &BTreeSet<T>, set2: &BTreeSet<T>) -> BTreeSet<T> {
    let mut result = set1.clone();
    for item in set2.iter() {
      result.insert(item.clone());
    }
    result
  }

  /// Helper function to get the intersection of two sets
  fn intersection<T: Ord + Clone>(set1: &BTreeSet<T>, set2: &BTreeSet<T>) -> BTreeSet<T> {
    let mut result = BTreeSet::new();
    for item in set1.iter() {
      if set2.contains(item) {
        result.insert(item.clone());
      }
    }
    result
  }

  /// Helper function to filter elements in a BTreeSet
  fn filter_even(set: &BTreeSet<i32>) -> BTreeSet<i32> {
    set.iter().filter(|&x| x % 2 == 0).cloned().collect()
  }

  /// Helper function to double elements in a BTreeSet
  fn double_elements(set: &BTreeSet<i32>) -> BTreeSet<i32> {
    set.iter().map(|&x| x * 2).collect()
  }

  #[test]
  fn test_identity_law() {
    // Create a test BTreeSet
    let set = to_btreeset(vec![1, 2, 3]);

    // Create the identity morphism
    let id = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::id();

    // Apply it to the set
    let result = id.apply(set.clone());

    // Identity should preserve the original set
    assert_eq!(result, set);
  }

  #[test]
  fn test_composition_law() {
    // Create a test BTreeSet
    let set = to_btreeset(vec![1, 2, 3, 4]);

    // Create two morphisms: filter even numbers, then double them
    let f = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(filter_even);
    let g = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(double_elements);

    // Compose them
    let f_then_g =
      <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::compose(f.clone(), g.clone());

    // Apply the composed function
    let result = f_then_g.apply(set.clone());

    // Manually verify the expected result
    let filtered = filter_even(&set);
    let expected = double_elements(&filtered);

    assert_eq!(result, expected);

    // The result should be {4, 8} (even numbers doubled)
    let expected_manual = to_btreeset(vec![4, 8]);
    assert_eq!(result, expected_manual);
  }

  #[test]
  fn test_set_operations() {
    // Create test BTreeSets
    let set1 = to_btreeset(vec![1, 2, 3]);
    let set2 = to_btreeset(vec![3, 4, 5]);

    // Create morphisms for union and intersection
    let union_morphism = <BTreeSetCategory as Category<
      (BTreeSet<i32>, BTreeSet<i32>),
      BTreeSet<i32>,
    >>::arr(|(s1, s2)| union(s1, s2));

    let intersection_morphism = <BTreeSetCategory as Category<
      (BTreeSet<i32>, BTreeSet<i32>),
      BTreeSet<i32>,
    >>::arr(|(s1, s2)| intersection(s1, s2));

    // Apply the morphisms
    let union_result = union_morphism.apply((set1.clone(), set2.clone()));
    let intersection_result = intersection_morphism.apply((set1.clone(), set2.clone()));

    // Expected results
    let expected_union = to_btreeset(vec![1, 2, 3, 4, 5]);
    let expected_intersection = to_btreeset(vec![3]);

    // Verify the results
    assert_eq!(union_result, expected_union);
    assert_eq!(intersection_result, expected_intersection);
  }

  #[test]
  fn test_first() {
    // Create a test BTreeSet
    let set = to_btreeset(vec![1, 2]);

    // Create a value to pair with
    let value = "test".to_string();

    // Create a morphism and its "first" version
    let f = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(double_elements);
    let first_f = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::first(f);

    // Apply the first morphism to a pair
    let result = first_f.apply((set.clone(), value.clone()));

    // Verify the result
    assert_eq!(result.0, double_elements(&set));
    assert_eq!(result.1, value);
  }

  #[test]
  fn test_second() {
    // Create a test BTreeSet
    let set = to_btreeset(vec![1, 2]);

    // Create a value to pair with
    let value = "test".to_string();

    // Create a morphism and its "second" version
    let f = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(double_elements);
    let second_f = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::second(f);

    // Apply the second morphism to a pair
    let result = second_f.apply((value.clone(), set.clone()));

    // Verify the result
    assert_eq!(result.0, value);
    assert_eq!(result.1, double_elements(&set));
  }

  #[test]
  fn test_arr() {
    // Create a test BTreeSet
    let set = to_btreeset(vec![1, 2, 3]);

    // Create a morphism using arr
    let double_morphism =
      <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(double_elements);

    // Apply the morphism
    let result = double_morphism.apply(set.clone());

    // Verify the result
    assert_eq!(result, double_elements(&set));
  }

  #[test]
  fn test_with_different_types() {
    // Create a test BTreeSet with strings
    let set = to_btreeset(vec!["hello".to_string(), "world".to_string()]);

    // Create a function that transforms strings
    fn to_uppercase(set: &BTreeSet<String>) -> BTreeSet<String> {
      set.iter().map(|s| s.to_uppercase()).collect()
    }

    // Create a morphism using arr
    let uppercase_morphism =
      <BTreeSetCategory as Category<BTreeSet<String>, BTreeSet<String>>>::arr(to_uppercase);

    // Apply the morphism
    let result = uppercase_morphism.apply(set.clone());

    // Expected result
    let expected = to_btreeset(vec!["HELLO".to_string(), "WORLD".to_string()]);

    // Verify the result
    assert_eq!(result, expected);
  }

  #[test]
  fn test_category_laws() {
    // Create a test BTreeSet
    let set = to_btreeset(vec![1, 2, 3, 4]);

    // Create morphisms
    let f = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(filter_even);
    let g = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(double_elements);
    // Create a third morphism - let's use a function to negate all elements
    let h =
      <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(|s: &BTreeSet<i32>| {
        s.iter().map(|&x| -x).collect::<BTreeSet<i32>>()
      });

    // Test associativity: (f . g) . h = f . (g . h)
    let fg =
      <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::compose(f.clone(), g.clone());
    let fg_h = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::compose(fg, h.clone());

    let gh = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::compose(g, h);
    let f_gh = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::compose(f, gh);

    // Apply both compositions
    let result1 = fg_h.apply(set.clone());
    let result2 = f_gh.apply(set.clone());

    // The results should be the same
    assert_eq!(result1, result2);
  }

  proptest! {
    #[test]
    fn prop_identity_law(
      elements in prop::collection::vec(1..100i32, 0..10)
    ) {
      let set: BTreeSet<i32> = elements.into_iter().collect();
      let id = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::id();
      let result = id.apply(set.clone());
      prop_assert_eq!(result, set);
    }

    #[test]
    fn prop_composition_preserves_structure(
      elements in prop::collection::vec(1..100i32, 0..10)
    ) {
      let set: BTreeSet<i32> = elements.into_iter().collect();

      // Create morphisms
      let f = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(filter_even);
      let g = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::arr(double_elements);

      // Compose and apply
      let composed = <BTreeSetCategory as Category<BTreeSet<i32>, BTreeSet<i32>>>::compose(f.clone(), g.clone());
      let composed_result = composed.apply(set.clone());

      // Apply individually
      let f_result = f.apply(set.clone());
      let expected = g.apply(f_result);

      prop_assert_eq!(composed_result, expected);
    }
  }
}
