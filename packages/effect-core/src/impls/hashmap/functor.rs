use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, A> Functor<A> for HashMap<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  type HigherSelf<B: CloneableThreadSafe> = HashMap<K, B>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = HashMap::with_capacity(self.len());
    for (k, a) in self {
      result.insert(k.clone(), f(&a));
    }
    result
  }
}
