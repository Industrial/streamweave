use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<T: CloneableThreadSafe> Functor<T> for Arc<T> {
  type HigherSelf<U: CloneableThreadSafe> = Arc<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    Arc::new(f(&*self))
  }
}
