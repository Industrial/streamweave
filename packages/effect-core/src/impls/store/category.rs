use crate::traits::category::Category;
use crate::types::store::Store;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

// Define the morphism type for Store
pub struct StoreMorphism<A: CloneableThreadSafe, B: CloneableThreadSafe> {
  f: Arc<dyn for<'a> Fn(&'a A) -> B + Send + Sync + 'static>,
}

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Clone for StoreMorphism<A, B> {
  fn clone(&self) -> Self {
    StoreMorphism {
      f: Arc::clone(&self.f),
    }
  }
}

impl<S: CloneableThreadSafe, T: CloneableThreadSafe, U: CloneableThreadSafe> Category<T, U>
  for Store<S, T>
{
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = StoreMorphism<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    StoreMorphism {
      f: Arc::new(|a| a.clone()),
    }
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    StoreMorphism {
      f: Arc::new(move |a| (g.f)(&(f.f)(a))),
    }
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    StoreMorphism {
      f: Arc::new(move |a| f(a)),
    }
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    StoreMorphism {
      f: Arc::new(move |pair| ((f.f)(&pair.0), pair.1.clone())),
    }
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    StoreMorphism {
      f: Arc::new(move |pair| (pair.0.clone(), (f.f)(&pair.1))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_id() {
    let id = <Store<i32, i32> as Category<i32, i32>>::id::<i32>();
    let value = 42;
    assert_eq!((id.f)(&value), value);
  }

  #[test]
  fn test_compose() {
    let f = <Store<i32, i32> as Category<i32, i32>>::arr(|x: &i32| x + 1);
    let g = <Store<i32, i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());

    let composed = <Store<i32, i32> as Category<i32, String>>::compose(f, g);
    assert_eq!((composed.f)(&5), "6");
  }

  #[test]
  fn test_arr() {
    let f = <Store<i32, i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());
    assert_eq!((f.f)(&42), "42");
  }

  #[test]
  fn test_first() {
    let f = <Store<i32, i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());
    let first = <Store<i32, i32> as Category<i32, String>>::first(f);

    let pair = (5, "test");
    let result = (first.f)(&pair);
    assert_eq!(result, ("5".to_string(), "test"));
  }

  #[test]
  fn test_second() {
    let f = <Store<i32, i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());
    let second = <Store<i32, i32> as Category<i32, String>>::second(f);

    let pair = ("test", 5);
    let result = (second.f)(&pair);
    assert_eq!(result, ("test", "5".to_string()));
  }
}
