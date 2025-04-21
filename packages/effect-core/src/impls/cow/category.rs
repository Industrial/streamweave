use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::borrow::Cow;
use std::borrow::ToOwned;
use std::sync::Arc;

/// A morphism for Cow that delegates to the inner type's morphism
#[derive(Clone)]
pub struct CowFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> CowFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    CowFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

/// Implementation of Category for Cow<'static, T> where T implements Category
impl<T, U, InnerCategory> Category<Cow<'static, T>, Cow<'static, U>> for Cow<'static, InnerCategory>
where
  T: CloneableThreadSafe + ToOwned + 'static,
  U: CloneableThreadSafe + ToOwned + 'static,
  InnerCategory: Category<T, U> + ToOwned + Clone + 'static,
  <T as ToOwned>::Owned: CloneableThreadSafe,
  <U as ToOwned>::Owned: CloneableThreadSafe,
  <InnerCategory as ToOwned>::Owned: CloneableThreadSafe,
{
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = CowFn<A, B>;

  /// Identity morphism that preserves Cow values
  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    CowFn::new(|x| x)
  }

  /// Compose two morphisms, delegating to the underlying category composition
  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    CowFn::new(move |x| g.apply(f.apply(x)))
  }

  /// Lift a function to a morphism
  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'b> Fn(&'b A) -> B + CloneableThreadSafe,
  {
    CowFn::new(move |x| f(&x))
  }

  /// Apply a morphism to the first component of a pair
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    CowFn::new(move |(a, c)| (f.apply(a), c))
  }

  /// Apply a morphism to the second component of a pair
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    CowFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

// Specialized implementation for Cow<'static, str>
impl Category<Cow<'static, str>, Cow<'static, str>> for Cow<'static, str> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = CowFn<A, B>;

  /// Identity morphism that preserves Cow<str> values
  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    CowFn::new(|x| x)
  }

  /// Compose two morphisms for Cow<str>
  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    CowFn::new(move |x| g.apply(f.apply(x)))
  }

  /// Lift a function to a morphism for Cow<str>
  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'b> Fn(&'b A) -> B + CloneableThreadSafe,
  {
    CowFn::new(move |x| f(&x))
  }

  /// Apply a morphism to the first component of a pair
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    CowFn::new(move |(a, c)| (f.apply(a), c))
  }

  /// Apply a morphism to the second component of a pair
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    CowFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test function to convert Cow<str> to uppercase
  fn to_uppercase(s: &Cow<'_, str>) -> Cow<'static, str> {
    match s {
      Cow::Borrowed(b) => Cow::Owned(b.to_uppercase()),
      Cow::Owned(o) => Cow::Owned(o.to_uppercase()),
    }
  }

  // Test function to concatenate a suffix to a Cow<str>
  fn add_suffix(s: &Cow<'_, str>) -> Cow<'static, str> {
    match s {
      Cow::Borrowed(b) => Cow::Owned(format!("{}_suffix", b)),
      Cow::Owned(o) => Cow::Owned(format!("{}_suffix", o)),
    }
  }

  #[test]
  fn test_identity_law_borrowed() {
    let cow: Cow<'static, str> = Cow::Borrowed("test");
    let id = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::id();
    let result = id.apply(cow.clone());
    assert_eq!(result, cow);
  }

  #[test]
  fn test_identity_law_owned() {
    let cow: Cow<'static, str> = Cow::Owned("test".to_string());
    let id = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::id();
    let result = id.apply(cow.clone());
    assert_eq!(result, cow);
  }

  #[test]
  fn test_composition_borrowed() {
    let cow: Cow<'static, str> = Cow::Borrowed("test");

    // Create individual morphisms
    let uppercase_fn =
      <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(to_uppercase);
    let suffix_fn =
      <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(add_suffix);

    // Compose them
    let composed = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::compose(
      uppercase_fn.clone(),
      suffix_fn.clone(),
    );

    // Apply the composed function
    let result = composed.apply(cow.clone());

    // Verify against manually applying each function
    let expected = suffix_fn.apply(uppercase_fn.apply(cow));

    assert_eq!(result, expected);
    assert_eq!(result, Cow::<str>::Owned("TEST_suffix".to_string()));
  }

  #[test]
  fn test_composition_owned() {
    let cow: Cow<'static, str> = Cow::Owned("test".to_string());

    // Create individual morphisms
    let uppercase_fn =
      <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(to_uppercase);
    let suffix_fn =
      <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(add_suffix);

    // Compose them
    let composed = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::compose(
      uppercase_fn.clone(),
      suffix_fn.clone(),
    );

    // Apply the composed function
    let result = composed.apply(cow.clone());

    // Verify against manually applying each function
    let expected = suffix_fn.apply(uppercase_fn.apply(cow));

    assert_eq!(result, expected);
    assert_eq!(result, Cow::<str>::Owned("TEST_suffix".to_string()));
  }

  #[test]
  fn test_first() {
    let cow: Cow<'static, str> = Cow::Borrowed("test");
    let value = 42;

    // Create a morphism and its "first" version
    let f =
      <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(to_uppercase);
    let first_f = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::first(f);

    // Apply the first morphism to a pair
    let result = first_f.apply((cow.clone(), value));

    // Verify the result
    assert_eq!(result.0, to_uppercase(&cow));
    assert_eq!(result.1, value);
  }

  #[test]
  fn test_second() {
    let cow: Cow<'static, str> = Cow::Borrowed("test");
    let value = 42;

    // Create a morphism and its "second" version
    let f =
      <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(to_uppercase);
    let second_f = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::second(f);

    // Apply the second morphism to a pair
    let result = second_f.apply((value, cow.clone()));

    // Verify the result
    assert_eq!(result.0, value);
    assert_eq!(result.1, to_uppercase(&cow));
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(s in "\\PC*") {
      let cow: Cow<'static, str> = Cow::Owned(s);
      let id = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::id();
      let result = id.apply(cow.clone());
      prop_assert_eq!(result, cow);
    }

    #[test]
    fn prop_composition_associativity(s in "\\PC*") {
      let cow: Cow<'static, str> = Cow::Owned(s);

      // Create morphisms
      let f = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(to_uppercase);
      let g = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(add_suffix);
      // Third morphism - convert to lowercase (assuming input is already uppercase)
      let h = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::arr(|s: &Cow<'_, str>| {
        match s {
          Cow::Borrowed(b) => Cow::<str>::Owned(b.to_lowercase()),
          Cow::Owned(o) => Cow::<str>::Owned(o.to_lowercase()),
        }
      });

      // Test (f . g) . h = f . (g . h)
      let fg = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::compose(f.clone(), g.clone());
      let fg_h = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::compose(fg, h.clone());

      let gh = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::compose(g, h);
      let f_gh = <Cow<'static, str> as Category<Cow<'static, str>, Cow<'static, str>>>::compose(f, gh);

      // Apply both compositions
      let result1 = fg_h.apply(cow.clone());
      let result2 = f_gh.apply(cow);

      // The results should be the same
      prop_assert_eq!(result1, result2);
    }
  }
}
