use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashSet;
use std::hash::Hash;

impl<T: Eq + Hash + CloneableThreadSafe> Foldable<T> for HashSet<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    let mut acc = init;
    for x in self {
      acc = f(acc, &x);
    }
    acc
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // For HashSet, fold_right doesn't have a natural order, so we collect into a Vec first
    let entries: Vec<T> = self.into_iter().collect();
    let mut acc = init;
    for x in entries.into_iter().rev() {
      acc = f(&x, acc);
    }
    acc
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let mut iter = self.into_iter();
    let first = iter.next().unwrap();
    let mut acc = first;

    for x in iter {
      acc = f(&acc, &x);
    }

    Some(acc)
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    // For HashSet, reduce_right doesn't have a natural order, so we collect into a Vec first
    let entries: Vec<T> = self.into_iter().collect();
    let mut iter = entries.iter().rev();

    let first = iter.next().unwrap();
    let mut acc = first.clone();

    for x in iter {
      acc = f(x, &acc);
    }

    Some(acc)
  }
}
