use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Applicative<A> for Box<A> {
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    Box::new(value)
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let a = self;
    let mut f_val = *f;
    Box::new(f_val(&a))
  }
}
