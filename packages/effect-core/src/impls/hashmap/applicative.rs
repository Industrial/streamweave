use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, A> Applicative<A> for HashMap<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn pure<B>(_value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    // Since we can't create a HashMap with no keys, we return an empty HashMap
    HashMap::new()
  }

  fn ap<B, F>(self, fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = HashMap::new();
    for (k, a) in &self {
      if let Some(mut f) = fs.get(k).cloned() {
        result.insert(k.clone(), f(a));
      }
    }
    result
  }
}
