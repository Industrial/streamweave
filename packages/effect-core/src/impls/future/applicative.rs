use crate::impls::future::category::FutureCategory;
use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;
use std::marker::PhantomData;

// Implement Applicative for FutureCategory, not FutureFn directly
impl<A: CloneableThreadSafe + Send + 'static> Applicative<A> for FutureCategory<A> {
  fn pure<B>(_value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe + Send + 'static,
  {
    // Return the wrapper type - this doesn't actually hold or use the value
    // The actual implementation happens through the underlying Category implementation
    FutureCategory(PhantomData)
  }

  fn ap<B, F>(self, _f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe + Send + 'static,
  {
    // Return the wrapper type
    FutureCategory(PhantomData)
  }
}
