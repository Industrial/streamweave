//! MonadPlus trait and implementations.
//!
//! A MonadPlus is a monad that also supports alternative operations.
//! It combines the properties of a monad with those of an alternative.

use super::alternative::Alternative;
use super::monad::Monad;
use std::marker::PhantomData;
use std::sync::Arc;

/// A MonadPlus is a monad that also supports alternative operations.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter A must implement Send + Sync + 'static
/// - The functions f and g must implement Send + Sync + 'static
pub trait MonadPlus<A: Send + Sync + 'static>: Monad<A> + Alternative<A> {
  /// The empty value for this monad.
  fn empty() -> Self;

  /// Combines two monadic values, preferring the first one if both are non-empty.
  fn mplus(self, other: Self) -> Self;

  /// Filters a monadic value based on a predicate.
  fn mfilter<F>(self, f: F) -> Self
  where
    F: Fn(&A) -> bool + Send + Sync + 'static,
  {
    self.bind(move |x| if f(&x) { Self::pure(x) } else { Self::empty() })
  }
}

/// Implementation of Alternative for Vec.
impl<A: Send + Sync + 'static> Alternative<A> for Vec<A> {
  fn empty() -> Self {
    Vec::new()
  }

  fn alt(self, other: Self) -> Self {
    let mut result = self;
    result.extend(other);
    result
  }
}

/// Implementation of Monad for Vec.
impl<A: Send + Sync + 'static> Monad<A> for Vec<A> {
  type MonadSelf<B: Send + Sync + 'static> = Vec<B>;

  fn bind<B: Send + Sync + 'static, F>(self, mut f: F) -> Vec<B>
  where
    F: FnMut(A) -> Vec<B> + Send + Sync + 'static,
  {
    self.into_iter().flat_map(f).collect()
  }
}

/// Implementation of MonadPlus for Vec.
impl<A: Send + Sync + 'static> MonadPlus<A> for Vec<A> {
  fn empty() -> Self {
    Vec::new()
  }

  fn mplus(self, other: Self) -> Self {
    let mut result = self;
    result.extend(other);
    result
  }
}

/// Implementation of MonadPlus for Option.
impl<A: Send + Sync + 'static> MonadPlus<A> for Option<A> {
  fn empty() -> Self {
    None
  }

  fn mplus(self, other: Self) -> Self {
    self.or(other)
  }
}

/// Implementation of MonadPlus for Result.
impl<A: Send + Sync + 'static, E: Send + Sync + 'static> MonadPlus<A> for Result<A, E> {
  fn empty() -> Self {
    Err(Arc::new(PhantomData::<E>))
  }

  fn mplus(self, other: Self) -> Self {
    self.or(other)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn test_option_monad_plus(
      x in any::<i32>(),
      y in any::<i32>()
    ) {
      let empty = Option::<i32>::empty();
      assert_eq!(empty, None);

      let some_x = Some(x);
      let some_y = Some(y);
      assert_eq!(some_x.mplus(some_y), some_x);
      assert_eq!(empty.mplus(some_x), some_x);
      assert_eq!(some_x.mplus(empty), some_x);

      let filtered = some_x.mfilter(|&x| x > 0);
      assert_eq!(filtered, if x > 0 { some_x } else { empty });
    }

    #[test]
    fn test_vec_monad_plus(
      xs in prop::collection::vec(any::<i32>(), 0..10),
      ys in prop::collection::vec(any::<i32>(), 0..10)
    ) {
      let empty = Vec::<i32>::empty();
      assert!(empty.is_empty());

      let mut expected = xs.clone();
      expected.extend(ys.clone());
      assert_eq!(xs.mplus(ys), expected);
      assert_eq!(empty.mplus(xs.clone()), xs);
      assert_eq!(xs.mplus(empty), xs);

      let filtered = xs.mfilter(|&x| x > 0);
      assert_eq!(filtered, xs.into_iter().filter(|&x| x > 0).collect());
    }

    #[test]
    fn test_result_monad_plus(
      x in any::<i32>(),
      y in any::<i32>()
    ) {
      let empty = Result::<i32, &str>::empty();
      assert!(empty.is_err());

      let ok_x = Ok(x);
      let ok_y = Ok(y);
      assert_eq!(ok_x.mplus(ok_y), ok_x);
      assert_eq!(empty.mplus(ok_x), ok_x);
      assert_eq!(ok_x.mplus(empty), ok_x);

      let filtered = ok_x.mfilter(|&x| x > 0);
      assert_eq!(filtered, if x > 0 { ok_x } else { empty });
    }
  }
}
