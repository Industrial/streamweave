use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::cmp::Ord;
use std::collections::BTreeMap;

impl<K: Ord + CloneableThreadSafe, V: CloneableThreadSafe> Foldable<(K, V)> for BTreeMap<K, V> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a (K, V)) -> A + CloneableThreadSafe,
  {
    let mut acc = init;
    for (k, v) in self.into_iter() {
      let pair = (k, v);
      acc = f(acc, &pair);
    }
    acc
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a (K, V), A) -> A + CloneableThreadSafe,
  {
    // For BTreeMap, fold_right naturally uses the reverse iteration order
    let entries: Vec<(K, V)> = self.into_iter().collect();
    let mut acc = init;
    for pair in entries.into_iter().rev() {
      acc = f(&pair, acc);
    }
    acc
  }

  fn reduce<F>(self, mut f: F) -> Option<(K, V)>
  where
    F: for<'a, 'b> FnMut(&'a (K, V), &'b (K, V)) -> (K, V) + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let mut iter = self.into_iter();
    let first = iter.next().unwrap();
    let mut acc = first;

    for pair in iter {
      acc = f(&acc, &pair);
    }

    Some(acc)
  }

  fn reduce_right<F>(self, mut f: F) -> Option<(K, V)>
  where
    F: for<'a, 'b> FnMut(&'a (K, V), &'b (K, V)) -> (K, V) + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    // For BTreeMap, reduce_right uses the reverse order
    let entries: Vec<(K, V)> = self.into_iter().collect();
    let mut iter = entries.iter().rev();

    let first = iter.next().unwrap();
    let mut acc = (first.0.clone(), first.1.clone());

    for pair in iter {
      acc = f(pair, &acc);
    }

    Some(acc)
  }
}
