use super::bifunctor::Bifunctor;
use super::contravariant::Contravariant;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait Category {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A>;

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C>;

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static;

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)>;

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)>;
}

pub struct Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  f: Arc<dyn Fn(A) -> B + Send + Sync + 'static>,
  _phantom: PhantomData<(A, B)>,
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for Morphism<A, B> {
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Morphism<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Self {
      f: Arc::new(f),
      _phantom: PhantomData,
    }
  }

  pub fn apply(&self, x: A) -> B {
    (self.f)(x)
  }
}

impl Category for Morphism<(), ()> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Morphism<A, B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Morphism::new(|x| x)
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    Morphism::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Morphism::new(f)
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Morphism::new(move |(a, c)| (f.apply(a), c))
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Morphism::new(move |(c, a)| (c, f.apply(a)))
  }
}

impl Bifunctor for Morphism<(), ()> {
  fn bimap<X, Y, Z, W, F, G>(f: F, g: G) -> Self::Morphism<(X, Y), (Z, W)>
  where
    X: Send + Sync + 'static,
    Y: Send + Sync + 'static,
    Z: Send + Sync + 'static,
    W: Send + Sync + 'static,
    F: Fn(X) -> Z + Send + Sync + 'static,
    G: Fn(Y) -> W + Send + Sync + 'static,
  {
    Morphism::new(move |(x, y)| (f(x), g(y)))
  }
}

impl Contravariant for Morphism<(), ()> {
  fn contramap<A, B, F>(f: F) -> Self::Morphism<B, A>
  where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    F: Fn(B) -> A + Send + Sync + 'static,
  {
    Morphism::new(f)
  }
}

impl<U> Category for Option<U> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Option<B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    None
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    f.and_then(|_| g)
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(_f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    None
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    None
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    None
  }
}

impl<U, E: Send + Sync + Default + 'static> Category for Result<U, E> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Result<B, E>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Err(E::default())
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    f.and_then(|_| g)
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(_f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Err(E::default())
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Err(E::default())
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Err(E::default())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn test_category_composition(a: i32) {
      let f = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_add(1).unwrap_or(i32::MAX));
      let g = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let composed = <Morphism<(), ()> as Category>::compose(f, g);
      let expected = a.checked_add(1)
        .and_then(|x| x.checked_mul(2))
        .unwrap_or(i32::MAX);
      assert_eq!(composed.apply(a), expected);
    }

    #[test]
    fn test_category_identity(a: i32) {
      let id = <Morphism<(), ()> as Category>::id::<i32>();
      assert_eq!(id.apply(a), a);
    }

    #[test]
    fn test_category_laws(a: i32) {
      // Test identity laws
      let f = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let id = <Morphism<(), ()> as Category>::id::<i32>();

      let f_id = <Morphism<(), ()> as Category>::compose(f.clone(), id.clone());
      let id_f = <Morphism<(), ()> as Category>::compose(id, f.clone());

      let expected = a.checked_mul(2).unwrap_or(i32::MAX);
      assert_eq!(f_id.apply(a), expected);
      assert_eq!(id_f.apply(a), expected);

      // Test associativity
      let g = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_add(1).unwrap_or(i32::MAX));
      let h = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_mul(3).unwrap_or(i32::MAX));

      let comp1 = <Morphism<(), ()> as Category>::compose(
        <Morphism<(), ()> as Category>::compose(f.clone(), g.clone()),
        h.clone()
      );
      let comp2 = <Morphism<(), ()> as Category>::compose(
        f,
        <Morphism<(), ()> as Category>::compose(g, h)
      );

      let expected = a.checked_mul(2)
        .and_then(|x| x.checked_add(1))
        .and_then(|x| x.checked_mul(3))
        .unwrap_or(i32::MAX);
      assert_eq!(comp1.apply(a), expected);
      assert_eq!(comp2.apply(a), expected);
    }

    #[test]
    fn test_category_arr(a: i32) {
      let f = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let expected = a.checked_mul(2).unwrap_or(i32::MAX);
      assert_eq!(f.apply(a), expected);
    }

    #[test]
    fn test_category_first(a: i32, b: i32) {
      let f = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let first = <Morphism<(), ()> as Category>::first(f);
      let expected = (a.checked_mul(2).unwrap_or(i32::MAX), b);
      assert_eq!(first.apply((a, b)), expected);
    }

    #[test]
    fn test_category_second(a: i32, b: i32) {
      let f = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let second = <Morphism<(), ()> as Category>::second(f);
      let expected = (a, b.checked_mul(2).unwrap_or(i32::MAX));
      assert_eq!(second.apply((a, b)), expected);
    }

    #[test]
    fn test_category_first_second_laws(a: i32, b: i32, _c: i32) {
      let f = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let g = <Morphism<(), ()> as Category>::arr(|x: i32| x.checked_add(1).unwrap_or(i32::MAX));

      // Test first . second = second . first
      let first_then_second = <Morphism<(), ()> as Category>::compose(
        <Morphism<(), ()> as Category>::first(f.clone()),
        <Morphism<(), ()> as Category>::second(g.clone())
      );

      let second_then_first = <Morphism<(), ()> as Category>::compose(
        <Morphism<(), ()> as Category>::second(g),
        <Morphism<(), ()> as Category>::first(f)
      );

      let input = (a, b);
      let result1 = first_then_second.apply(input.clone());
      let result2 = second_then_first.apply(input);
      assert_eq!(result1, result2);
    }
  }
}
