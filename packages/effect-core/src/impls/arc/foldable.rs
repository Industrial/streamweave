use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<T: CloneableThreadSafe> Foldable<T> for Arc<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    // For Arc, fold over the referenced value
    f(init, &self)
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // For Arc, fold_right over the referenced value
    f(&self, init)
  }

  fn reduce<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Arc, we need to clone the inner value
    Some((*self).clone())
  }

  fn reduce_right<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Arc, we need to clone the inner value
    Some((*self).clone())
  }
}
