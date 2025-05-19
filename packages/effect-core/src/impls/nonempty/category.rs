use crate::traits::category::Category;
use crate::types::nonempty::NonEmpty;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

// Define a simple identity morphism type for NonEmpty using Arc for clonability
pub struct NonEmptyMorphism<A: CloneableThreadSafe, B: CloneableThreadSafe> {
  f: Arc<dyn for<'a> Fn(&'a A) -> B + Send + Sync + 'static>,
}

// Manual impl of Clone because we can't derive it for a trait object
impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Clone for NonEmptyMorphism<A, B> {
  fn clone(&self) -> Self {
    NonEmptyMorphism {
      f: Arc::clone(&self.f),
    }
  }
}

impl<T: CloneableThreadSafe, U: CloneableThreadSafe> Category<T, U> for NonEmpty<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = NonEmptyMorphism<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    NonEmptyMorphism {
      f: Arc::new(|a| a.clone()),
    }
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    NonEmptyMorphism {
      f: Arc::new(move |a| (g.f)(&(f.f)(a))),
    }
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    NonEmptyMorphism {
      f: Arc::new(move |a| f(a)),
    }
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    NonEmptyMorphism {
      f: Arc::new(move |pair| ((f.f)(&pair.0), pair.1.clone())),
    }
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    NonEmptyMorphism {
      f: Arc::new(move |pair| (pair.0.clone(), (f.f)(&pair.1))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::nonempty::NonEmpty;

  #[test]
  fn test_id() {
    let id = <NonEmpty<i32> as Category<i32, i32>>::id::<i32>();
    let value = 42;
    assert_eq!((id.f)(&value), value);
  }

  #[test]
  fn test_compose() {
    let f = <NonEmpty<i32> as Category<i32, i32>>::arr(|x: &i32| x + 1);
    let g = <NonEmpty<i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());

    let composed = <NonEmpty<i32> as Category<i32, String>>::compose(f, g);
    assert_eq!((composed.f)(&5), "6");
  }

  #[test]
  fn test_arr() {
    let f = <NonEmpty<i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());
    assert_eq!((f.f)(&42), "42");
  }

  #[test]
  fn test_first() {
    let f = <NonEmpty<i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());
    let first = <NonEmpty<i32> as Category<i32, String>>::first(f);

    let pair = (5, "test");
    let result = (first.f)(&pair);
    assert_eq!(result, ("5".to_string(), "test"));
  }

  #[test]
  fn test_second() {
    let f = <NonEmpty<i32> as Category<i32, String>>::arr(|x: &i32| x.to_string());
    let second = <NonEmpty<i32> as Category<i32, String>>::second(f);

    let pair = ("test", 5);
    let result = (second.f)(&pair);
    assert_eq!(result, ("test", "5".to_string()));
  }
}
