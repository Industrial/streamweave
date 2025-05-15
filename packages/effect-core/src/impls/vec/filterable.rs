use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Filterable<A> for Vec<A> {
  type Filtered<B: CloneableThreadSafe> = Vec<B>;

  fn filter_map<B, F>(mut self, mut f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = Vec::with_capacity(self.len());
    for item in self.drain(..) {
      if let Some(mapped) = f(&item) {
        result.push(mapped);
      }
    }
    result
  }

  // We can override the default filter implementation for better performance
  fn filter<F>(mut self, mut predicate: F) -> Self::Filtered<A>
  where
    F: for<'a> FnMut(&'a A) -> bool + CloneableThreadSafe,
    A: Clone,
  {
    let mut result = Vec::with_capacity(self.len());
    for item in self.drain(..) {
      if predicate(&item) {
        result.push(item);
      }
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_filter_map_empty() {
    let vec: Vec<i32> = vec![];
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = Filterable::filter_map(vec, f);
    assert_eq!(result, Vec::<String>::new());
  }

  #[test]
  fn test_filter_map_all_some() {
    let vec = vec![20, 30, 40];
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = Filterable::filter_map(vec, f);
    assert_eq!(result, vec!["20".to_string(), "30".to_string(), "40".to_string()]);
  }

  #[test]
  fn test_filter_map_some_none() {
    let vec = vec![5, 15, 25];
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = Filterable::filter_map(vec, f);
    assert_eq!(result, vec!["15".to_string(), "25".to_string()]);
  }

  #[test]
  fn test_filter_map_all_none() {
    let vec = vec![1, 2, 3];
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = Filterable::filter_map(vec, f);
    assert_eq!(result, Vec::<String>::new());
  }

  #[test]
  fn test_filter_empty() {
    let vec: Vec<i32> = vec![];
    let predicate = |x: &i32| *x > 10;
    
    let result = Filterable::filter(vec, predicate);
    assert_eq!(result, Vec::<i32>::new());
  }

  #[test]
  fn test_filter_all_pass() {
    let vec = vec![20, 30, 40];
    let predicate = |x: &i32| *x > 10;
    
    let result = Filterable::filter(vec, predicate);
    assert_eq!(result, vec![20, 30, 40]);
  }

  #[test]
  fn test_filter_some_pass() {
    let vec = vec![5, 15, 25];
    let predicate = |x: &i32| *x > 10;
    
    let result = Filterable::filter(vec, predicate);
    assert_eq!(result, vec![15, 25]);
  }

  #[test]
  fn test_filter_none_pass() {
    let vec = vec![1, 2, 3];
    let predicate = |x: &i32| *x > 10;
    
    let result = Filterable::filter(vec, predicate);
    assert_eq!(result, Vec::<i32>::new());
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Identity law: filter_map(Some) == self
      let vec_clone = xs.clone();
      let identity = |val: &i32| Some(*val);
      
      let result = Filterable::filter_map(xs, identity);
      prop_assert_eq!(result, vec_clone);
    }

    #[test]
    fn prop_annihilation_law(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Annihilation law: filter_map(|_| None) == empty
      let none_fn = |_: &i32| None::<i32>;
      
      let result = Filterable::filter_map(xs, none_fn);
      prop_assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn prop_distributivity_law(xs in prop::collection::vec(any::<i32>(), 0..100), limit in 1..100i32) {
      // Distributivity law: filter_map(f).filter_map(g) == filter_map(|x| f(x).and_then(g))
      
      // Define filter functions
      let f = |val: &i32| if val.abs() % 2 == 0 { Some(*val) } else { None };
      let g = move |val: &i32| if val.abs() < limit { Some(val.to_string()) } else { None };
      
      // Apply filters sequentially
      let result1 = Filterable::filter_map(xs.clone(), f);
      let result1 = Filterable::filter_map(result1, g);
      
      // Apply composed filter
      let result2 = Filterable::filter_map(xs, move |val| {
        f(val).and_then(|v| g(&v))
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map(xs in prop::collection::vec(any::<i32>(), 0..100), threshold in 1..100i32) {
      // filter(p) == filter_map(|x| if p(x) { Some(x) } else { None })
      let predicate = move |val: &i32| val.abs() < threshold;
      
      let result1 = Filterable::filter(xs.clone(), predicate);
      let result2 = Filterable::filter_map(xs, move |val| {
        if predicate(val) { Some(*val) } else { None }
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_preserves_order(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Filtering should preserve the order of elements
      let predicate = |val: &i32| val % 2 == 0;
      
      let mut manual_result = Vec::new();
      for x in &xs {
        if predicate(x) {
          manual_result.push(*x);
        }
      }
      
      let result = Filterable::filter(xs, predicate);
      prop_assert_eq!(result, manual_result);
    }
  }
} 