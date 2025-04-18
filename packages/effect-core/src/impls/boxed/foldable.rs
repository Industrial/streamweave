use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Foldable<T> for Box<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    // For Box, extract the value and fold over it
    let value = *self;
    f(init, &value)
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // For Box, extract the value and fold_right over it
    let value = *self;
    f(&value, init)
  }

  fn reduce<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Box, reduce just extracts the value
    Some(*self)
  }

  fn reduce_right<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Box, reduce_right also just extracts the value
    Some(*self)
  }
}
