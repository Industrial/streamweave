use crate::impls::future::category::FutureCategory;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::marker::PhantomData;

// We need to implement Functor for FutureCategory<A>, not for FutureFn directly
impl<A: CloneableThreadSafe + Send + 'static> Functor<A> for FutureCategory<A> {
  type HigherSelf<B: CloneableThreadSafe> = FutureCategory<B>;

  fn map<B, F>(self, _f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe + Send + 'static,
  {
    // This is a bit of a dummy implementation since FutureCategory doesn't actually store data
    // It relies on the Category implementation for FutureCategory to actually do the mapping
    FutureCategory(PhantomData)
  }
}
