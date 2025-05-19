use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Filterable<A> for Option<A> {
  type Filtered<B: CloneableThreadSafe> = Option<B>;

  fn filter_map<B, F>(self, mut f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    match self {
      Some(a) => f(&a),
      None => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_filter_map_some() {
    let value = Some(42);
    let f = |x: &i32| if *x > 40 { Some(x.to_string()) } else { None };

    let result = Filterable::filter_map(value, f);
    assert_eq!(result, Some("42".to_string()));
  }

  #[test]
  fn test_filter_map_some_to_none() {
    let value = Some(30);
    let f = |x: &i32| if *x > 40 { Some(x.to_string()) } else { None };

    let result = Filterable::filter_map(value, f);
    assert_eq!(result, None);
  }

  #[test]
  fn test_filter_map_none() {
    let value: Option<i32> = None;
    let f = |x: &i32| if *x > 40 { Some(x.to_string()) } else { None };

    let result = Filterable::filter_map(value, f);
    assert_eq!(result, None);
  }

  #[test]
  fn test_filter_some_pass() {
    let value = Some(42);
    let predicate = |x: &i32| *x > 40;

    let result = Filterable::filter(value, predicate);
    assert_eq!(result, Some(42));
  }

  #[test]
  fn test_filter_some_fail() {
    let value = Some(30);
    let predicate = |x: &i32| *x > 40;

    let result = Filterable::filter(value, predicate);
    assert_eq!(result, None);
  }

  #[test]
  fn test_filter_none() {
    let value: Option<i32> = None;
    let predicate = |x: &i32| *x > 40;

    let result = Filterable::filter(value, predicate);
    assert_eq!(result, None);
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(x in any::<i32>()) {
      // Identity law: filter_map(Some) == self
      let opt = Some(x);
      let identity = |val: &i32| Some(*val);

      let result = Filterable::filter_map(opt.clone(), identity);
      prop_assert_eq!(result, opt);
    }

    #[test]
    fn prop_annihilation_law(x in any::<i32>()) {
      // Annihilation law: filter_map(|_| None) == None
      let opt = Some(x);
      let none_fn = |_: &i32| None::<i32>;

      let result = Filterable::filter_map(opt, none_fn);
      prop_assert_eq!(result, None);
    }

    #[test]
    fn prop_distributivity_law(x in any::<i32>(), limit in 1..100i32) {
      // Distributivity law: filter_map(f).filter_map(g) == filter_map(|x| f(x).and_then(g))
      let opt = Some(x);

      // Define filter functions
      let f = |val: &i32| if *val % 2 == 0 { Some(*val) } else { None };
      let g = move |val: &i32| if *val < limit { Some(val.to_string()) } else { None };

      // Apply filters sequentially
      let result1 = Filterable::filter_map(opt.clone(), f);
      let result1 = Filterable::filter_map(result1, g);

      // Apply composed filter
      let result2 = Filterable::filter_map(opt, move |val| {
        f(val).and_then(|v| g(&v))
      });

      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map(x in any::<i32>(), threshold in 1..100i32) {
      // filter(p) == filter_map(|x| if p(x) { Some(x) } else { None })
      let opt = Some(x);
      let predicate = move |val: &i32| *val < threshold;

      let result1 = Filterable::filter(opt.clone(), predicate);
      let result2 = Filterable::filter_map(opt, move |val| {
        if predicate(val) { Some(*val) } else { None }
      });

      prop_assert_eq!(result1, result2);
    }
  }
}
