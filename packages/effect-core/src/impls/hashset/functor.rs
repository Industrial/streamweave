use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashSet;
use std::hash::Hash;

impl<T: CloneableThreadSafe> Functor<T> for HashSet<T>
where
  T: Eq + Hash,
{
  type HigherSelf<U: CloneableThreadSafe> = HashSetWrapper<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // We'll create a vector to avoid requiring U: Eq + Hash
    let values: Vec<U> = self.into_iter().map(|x| f(&x)).collect();
    HashSetWrapper::new(values)
  }
}

// Wrapper struct to avoid requiring U: Eq + Hash in the trait definition
#[derive(Debug, Clone)]
pub struct HashSetWrapper<T: CloneableThreadSafe>(Vec<T>);

impl<T: CloneableThreadSafe> HashSetWrapper<T> {
  fn new(values: Vec<T>) -> Self {
    HashSetWrapper(values)
  }

  // Convert to a HashSet if T: Eq + Hash
  pub fn into_hashset(self) -> HashSet<T>
  where
    T: Eq + Hash,
  {
    self.0.into_iter().collect()
  }

  // Get the inner vector
  pub fn into_vec(self) -> Vec<T> {
    self.0
  }

  // Check if the wrapper contains a value
  pub fn contains(&self, value: &T) -> bool
  where
    T: PartialEq,
  {
    self.0.contains(value)
  }

  // Get the number of elements
  pub fn len(&self) -> usize {
    self.0.len()
  }

  // Check if the wrapper is empty
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }
}

impl<T: CloneableThreadSafe> CloneableThreadSafe for HashSetWrapper<T> {}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_functor_identity() {
    let mut set = HashSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    let mapped = Functor::map(set.clone(), |x| *x);
    let mapped_set = mapped.into_hashset();

    assert_eq!(mapped_set.len(), 3);
    assert!(mapped_set.contains(&1));
    assert!(mapped_set.contains(&2));
    assert!(mapped_set.contains(&3));
  }

  #[test]
  fn test_functor_composition() {
    let mut set = HashSet::new();
    set.insert(1);
    set.insert(2);

    let f = |x: &i32| x + 1;
    let g = |x: &i32| x * 2;

    let set_then_map = Functor::map(Functor::map(set.clone(), f), g).into_hashset();
    let composed = Functor::map(set, |x| g(&f(x))).into_hashset();

    // Since HashSet doesn't guarantee order, we need to check that both sets
    // contain the same elements
    assert_eq!(set_then_map.len(), composed.len());
    for item in set_then_map.iter() {
      assert!(composed.contains(item));
    }
  }

  proptest! {
    #[test]
    fn prop_functor_identity(xs: Vec<i32>) {
      let set: HashSet<_> = xs.into_iter().collect();
      let mapped = Functor::map(set.clone(), |v| *v).into_hashset();

      for v in set.iter() {
        prop_assert!(mapped.contains(v));
      }
      prop_assert_eq!(mapped.len(), set.len());
    }

    #[test]
    fn prop_functor_composition(xs: Vec<i32>) {
      let set: HashSet<_> = xs.into_iter().collect();
      let f = |v: &i32| *v + 1;
      let g = |v: &i32| *v * 2;

      let set_then_map = Functor::map(Functor::map(set.clone(), f), g).into_hashset();
      let composed = Functor::map(set, |v| g(&f(v))).into_hashset();

      // Check that both sets contain the same elements
      prop_assert_eq!(set_then_map.len(), composed.len());
      for item in set_then_map.iter() {
        prop_assert!(composed.contains(item));
      }
    }
  }
}
