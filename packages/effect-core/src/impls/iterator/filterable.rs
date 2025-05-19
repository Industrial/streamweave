use crate::impls::iterator::category::IteratorCategory;
use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::marker::PhantomData;

// Extension trait to make Iterator filtering more ergonomic
pub trait IteratorFilterableExt<A>: Iterator<Item = A>
where
  A: CloneableThreadSafe,
{
  fn filter_by<F>(self, predicate: F) -> std::iter::Filter<Self, F>
  where
    Self: Sized,
    F: FnMut(&A) -> bool;

  fn filter_map_by<B, F>(self, f: F) -> std::iter::FilterMap<Self, F>
  where
    Self: Sized,
    F: FnMut(A) -> Option<B>;
}

impl<I, A> IteratorFilterableExt<A> for I
where
  I: Iterator<Item = A>,
  A: CloneableThreadSafe,
{
  fn filter_by<F>(self, predicate: F) -> std::iter::Filter<Self, F>
  where
    Self: Sized,
    F: FnMut(&A) -> bool,
  {
    self.filter(predicate)
  }

  fn filter_map_by<B, F>(self, f: F) -> std::iter::FilterMap<Self, F>
  where
    Self: Sized,
    F: FnMut(A) -> Option<B>,
  {
    self.filter_map(f)
  }
}

// Implement Filterable for IteratorCategory
impl<A: CloneableThreadSafe + 'static> Filterable<A> for IteratorCategory<A> {
  type Filtered<B: CloneableThreadSafe> = IteratorCategory<B>;

  fn filter_map<B, F>(self, _f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // This is just a type-level implementation
    // The actual filtering happens via the extension trait
    IteratorCategory(PhantomData)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_filter_by_empty() {
    let iter = Vec::<i32>::new().into_iter();
    let result: Vec<i32> = iter.filter_by(|x| *x > 10).collect();
    assert_eq!(result, Vec::<i32>::new());
  }

  #[test]
  fn test_filter_by_all_pass() {
    let values = vec![20, 30, 40];
    let result: Vec<i32> = values.clone().into_iter().filter_by(|x| *x > 10).collect();
    assert_eq!(result, values);
  }

  #[test]
  fn test_filter_by_some_pass() {
    let iter = vec![5, 15, 25].into_iter();
    let result: Vec<i32> = iter.filter_by(|x| *x > 10).collect();
    assert_eq!(result, vec![15, 25]);
  }

  #[test]
  fn test_filter_by_none_pass() {
    let iter = vec![1, 2, 3].into_iter();
    let result: Vec<i32> = iter.filter_by(|x| *x > 10).collect();
    assert_eq!(result, Vec::<i32>::new());
  }

  #[test]
  fn test_filter_map_by_empty() {
    let iter = Vec::<i32>::new().into_iter();
    let result: Vec<String> = iter
      .filter_map_by(|x| if x > 10 { Some(x.to_string()) } else { None })
      .collect();
    assert_eq!(result, Vec::<String>::new());
  }

  #[test]
  fn test_filter_map_by_all_some() {
    let iter = vec![20, 30, 40].into_iter();
    let result: Vec<String> = iter
      .filter_map_by(|x| if x > 10 { Some(x.to_string()) } else { None })
      .collect();
    assert_eq!(
      result,
      vec!["20".to_string(), "30".to_string(), "40".to_string()]
    );
  }

  #[test]
  fn test_filter_map_by_some_none() {
    let iter = vec![5, 15, 25].into_iter();
    let result: Vec<String> = iter
      .filter_map_by(|x| if x > 10 { Some(x.to_string()) } else { None })
      .collect();
    assert_eq!(result, vec!["15".to_string(), "25".to_string()]);
  }

  #[test]
  fn test_filter_map_by_all_none() {
    let iter = vec![1, 2, 3].into_iter();
    let result: Vec<String> = iter
      .filter_map_by(|x| if x > 10 { Some(x.to_string()) } else { None })
      .collect();
    assert_eq!(result, Vec::<String>::new());
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_filter_preserves_order(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Filtering should preserve the order of elements
      let predicate = |val: &i32| val % 2 == 0;

      let mut manual_result = Vec::new();
      for x in &xs {
        if predicate(x) {
          manual_result.push(*x);
        }
      }

      let result: Vec<i32> = xs.into_iter().filter_by(predicate).collect();
      prop_assert_eq!(result, manual_result);
    }

    #[test]
    fn prop_filter_map_preserves_order(xs in prop::collection::vec(-100..100i32, 0..100)) {
      // filter_map should preserve the order of elements
      let f = |x: i32| if x % 2 == 0 { Some(x.saturating_mul(2)) } else { None };

      let mut manual_result = Vec::new();
      for x in &xs {
        if let Some(y) = f(*x) {
          manual_result.push(y);
        }
      }

      let result: Vec<i32> = xs.into_iter().filter_map_by(f).collect();
      prop_assert_eq!(result, manual_result);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map(xs in prop::collection::vec(any::<i32>(), 0..100), threshold in 1..100i32) {
      // filter(p) behaves the same as filter_map(|x| if p(x) { Some(x) } else { None })
      let predicate = move |val: &i32| val.abs() < threshold;
      let xs_clone = xs.clone();

      let result1: Vec<i32> = xs.into_iter()
        .filter_by(predicate)
        .collect();

      let result2: Vec<i32> = xs_clone.into_iter()
        .filter_map_by(move |val| {
          if predicate(&val) { Some(val) } else { None }
        })
        .collect();

      prop_assert_eq!(result1, result2);
    }
  }
}
