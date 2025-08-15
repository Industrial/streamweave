use crate::traits::functor::Functor;
use crate::types::store::Store;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<S: CloneableThreadSafe, A: CloneableThreadSafe> Functor<A> for Store<S, A> {
  type HigherSelf<U: CloneableThreadSafe> = Store<S, U>;

  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a A) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    let f = Arc::new(std::sync::Mutex::new(f));
    Store {
      peek: Arc::new(move |s| {
        let a = (self.peek)(s);
        let mut f_guard = f.lock().unwrap();
        f_guard(&a)
      }),
      pos: self.pos,
    }
  }

  fn map_owned<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(A) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized,
  {
    let f = Arc::new(std::sync::Mutex::new(f));
    Store {
      peek: Arc::new(move |s| {
        let a = (self.peek)(s);
        let mut f_guard = f.lock().unwrap();
        f_guard(a)
      }),
      pos: self.pos,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_map() {
    let store = Store::new(|x: &i32| x * 2, 5);
    let result = Functor::map(store, |x| x.to_string());

    assert_eq!(result.extract(), "10"); // (5 * 2).to_string() = "10"
    assert_eq!(result.peek_at(&8), "16"); // (8 * 2).to_string() = "16"
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(pos in any::<i32>()) {
      // Identity law: functor.map(|x| x) == functor
      let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);
      let mapped = Functor::map(store.clone(), |x| x.clone());

      prop_assert_eq!(mapped.extract(), store.extract());
      // Use wrapping_add to avoid overflow
      prop_assert_eq!(mapped.peek_at(&pos.wrapping_add(1)), store.peek_at(&pos.wrapping_add(1)));
    }

    #[test]
    fn prop_composition_law(pos in any::<i32>()) {
      // Composition law: functor.map(|x| g(f(x))) == functor.map(f).map(g)
      let store = Store::new(|x: &i32| x.wrapping_mul(2), pos);
      let f = |x: &i32| x.wrapping_add(10);
      let g = |x: &i32| x.to_string();

      // Apply f then g
      let result1 = Functor::map(store.clone(), f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let result2 = Functor::map(store, move |x| g(&f(x)));

      prop_assert_eq!(result1.extract(), result2.extract());
      // Use wrapping_add to avoid overflow
      prop_assert_eq!(result1.peek_at(&pos.wrapping_add(1)), result2.peek_at(&pos.wrapping_add(1)));
    }
  }
}
