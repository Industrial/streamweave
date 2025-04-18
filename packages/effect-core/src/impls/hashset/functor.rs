// Most imports can be removed since the implementation is commented out
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashSet;
use std::hash::Hash;

/* Commenting out the Functor implementation for HashSet as it requires the Category trait
   which we've already commented out for HashSet.

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
*/

// Wrapper struct to avoid requiring U: Eq + Hash in the trait definition
#[derive(Debug, Clone)]
pub struct HashSetWrapper<T: CloneableThreadSafe>(Vec<T>);

impl<T: CloneableThreadSafe> HashSetWrapper<T> {
  // Keep the implementation but mark it as deliberately unused
  #[allow(dead_code)]
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

/* Commenting out the CloneableThreadSafe implementation for HashSetWrapper as it's causing conflicts
impl<T: CloneableThreadSafe> CloneableThreadSafe for HashSetWrapper<T> {}
*/

#[cfg(test)]
mod tests {
  // Empty test module
}
