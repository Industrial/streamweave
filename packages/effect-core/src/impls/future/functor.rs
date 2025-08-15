use crate::impls::future::category::FutureCategory;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::marker::PhantomData;

// We need to implement Functor for FutureCategory<A>, not for FutureFn directly
impl<A: CloneableThreadSafe + Send + 'static> Functor<A> for FutureCategory<A> {
  type HigherSelf<U: CloneableThreadSafe> = FutureCategory<U>;

  fn map<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a A) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // This is a placeholder implementation since FutureCategory is just a proxy type
    // The actual mapping happens via the Category impl when using arr and apply
    FutureCategory(PhantomData)
  }

  fn map_owned<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(A) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized,
  {
    // This is a placeholder implementation since FutureCategory is just a proxy type
    // The actual mapping happens via the Category impl when using arr and apply
    FutureCategory(PhantomData)
  }
}
