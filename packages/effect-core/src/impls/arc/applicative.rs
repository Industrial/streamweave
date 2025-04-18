use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<A: CloneableThreadSafe> Applicative<A> for Arc<A> {
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    Arc::new(value)
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let f_clone = f.clone();
    let mut f_val = Arc::try_unwrap(f_clone).unwrap_or_else(|arc| (*arc).clone());
    Arc::new(f_val(&self))
  }
}
