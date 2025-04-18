use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Functor<T> for Vec<T> {
  type HigherSelf<U: CloneableThreadSafe> = Vec<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.into_iter().map(|x| f(&x)).collect()
  }
}
