use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Foldable<T> for Option<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    match self {
      Some(x) => f(init, &x),
      None => init,
    }
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    match self {
      Some(x) => f(&x, init),
      None => init,
    }
  }

  fn reduce<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Option, reduce is just identity since there is at most one element
    self
  }

  fn reduce_right<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Option, reduce_right is also identity
    self
  }
}
