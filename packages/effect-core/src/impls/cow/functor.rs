use crate::types::threadsafe::CloneableThreadSafe;
use std::borrow::Cow;
use std::borrow::ToOwned;
use std::marker::PhantomData;

/// A wrapper struct to store a mapping function for Cow values
#[derive(Clone)]
pub struct CowMapper<T, U, F>
where
  T: CloneableThreadSafe + ToOwned<Owned = T> + ?Sized + 'static,
  U: CloneableThreadSafe + ToOwned + ?Sized + 'static,
  F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
  <U as ToOwned>::Owned: CloneableThreadSafe,
{
  f: F,
  _phantom: PhantomData<(T, U)>,
}

impl<T, U, F> CowMapper<T, U, F>
where
  T: CloneableThreadSafe + ToOwned<Owned = T> + ?Sized + 'static,
  U: CloneableThreadSafe + ToOwned + ?Sized + 'static,
  F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
  <U as ToOwned>::Owned: CloneableThreadSafe,
{
  pub fn new(f: F) -> Self {
    CowMapper {
      f,
      _phantom: PhantomData,
    }
  }

  pub fn apply(&mut self, cow: Cow<'static, T>) -> Cow<'static, U> {
    match cow {
      Cow::Borrowed(borrowed) => Cow::Owned((self.f)(borrowed).to_owned()),
      Cow::Owned(owned) => Cow::Owned((self.f)(&owned).to_owned()),
    }
  }
}

// Extension trait to make mapping Cow values more ergonomic
pub trait CowFunctorExt<T>
where
  T: CloneableThreadSafe + ToOwned<Owned = T> + ?Sized + 'static,
{
  fn map<U, F>(self, f: F) -> Cow<'static, U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe + ToOwned + ?Sized + 'static,
    <U as ToOwned>::Owned: CloneableThreadSafe;
}

impl<T> CowFunctorExt<T> for Cow<'static, T>
where
  T: CloneableThreadSafe + ToOwned<Owned = T> + ?Sized + 'static,
{
  fn map<U, F>(self, mut f: F) -> Cow<'static, U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe + ToOwned + ?Sized + 'static,
    <U as ToOwned>::Owned: CloneableThreadSafe,
  {
    match self {
      Cow::Borrowed(borrowed) => Cow::Owned(f(borrowed).to_owned()),
      Cow::Owned(owned) => Cow::Owned(f(&owned).to_owned()),
    }
  }
}

// Special implementation for string slices
// We use a new trait to avoid the requirement that str: Clone
pub trait CowStrFunctorExt {
  fn map<U, F>(self, f: F) -> Cow<'static, U>
  where
    F: for<'a> FnMut(&'a str) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe + ToOwned + ?Sized + 'static,
    <U as ToOwned>::Owned: CloneableThreadSafe;
}

impl CowStrFunctorExt for Cow<'static, str> {
  fn map<U, F>(self, mut f: F) -> Cow<'static, U>
  where
    F: for<'a> FnMut(&'a str) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe + ToOwned + ?Sized + 'static,
    <U as ToOwned>::Owned: CloneableThreadSafe,
  {
    match self {
      Cow::Borrowed(borrowed) => Cow::Owned(f(borrowed).to_owned()),
      Cow::Owned(owned) => Cow::Owned(f(owned.as_str()).to_owned()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper functions for tests
  fn to_uppercase(s: &str) -> String {
    s.to_uppercase()
  }

  fn add_prefix(s: &str) -> String {
    format!("prefix_{}", s)
  }

  fn double_int(i: &i32) -> i32 {
    i.saturating_mul(2)
  }

  fn add_one(i: &i32) -> i32 {
    i.saturating_add(1)
  }

  #[test]
  fn test_map_borrowed_str() {
    let cow: Cow<'static, str> = Cow::Borrowed("hello");
    let result = cow.map(to_uppercase);

    assert_eq!(result, Cow::<'static, String>::Owned("HELLO".to_string()));
  }

  #[test]
  fn test_map_owned_str() {
    let cow: Cow<'static, str> = Cow::Owned("hello".to_string());
    let result = cow.map(to_uppercase);

    assert_eq!(result, Cow::<'static, String>::Owned("HELLO".to_string()));
  }

  #[test]
  fn test_map_borrowed_int() {
    let cow: Cow<'static, i32> = Cow::Borrowed(&42);
    let result = cow.map(double_int);

    assert_eq!(result, Cow::Owned(84));
  }

  #[test]
  fn test_map_owned_int() {
    let cow: Cow<'static, i32> = Cow::Owned(42);
    let result = cow.map(double_int);

    assert_eq!(result, Cow::Owned(84));
  }

  #[test]
  fn test_identity_law_borrowed() {
    let cow: Cow<'static, str> = Cow::Borrowed("test");
    let id = |x: &str| x.to_string();
    let result = cow.clone().map(id);

    assert_eq!(result, cow);
  }

  #[test]
  fn test_identity_law_owned() {
    let cow: Cow<'static, str> = Cow::Owned("test".to_string());
    let id = |x: &str| x.to_string();
    let result = cow.clone().map(id);

    assert_eq!(result, cow);
  }

  #[test]
  fn test_composition_law_str() {
    let cow: Cow<'static, str> = Cow::Borrowed("test");

    // Apply functions sequentially
    let result1 = cow
      .clone()
      .map(to_uppercase)
      .map(|s: &String| add_prefix(s.as_str()));

    // Apply composed function
    let result2 = cow.map(|s| add_prefix(&to_uppercase(s)));

    // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
    assert_eq!(result1, result2);
    assert_eq!(
      result1,
      Cow::<'static, String>::Owned("prefix_TEST".to_string())
    );
  }

  #[test]
  fn test_composition_law_int() {
    let cow: Cow<'static, i32> = Cow::Owned(10);

    // Apply functions sequentially
    let result1 = cow.clone().map(double_int).map(add_one);

    // Apply composed function
    let result2 = cow.map(|i| add_one(&double_int(i)));

    // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
    assert_eq!(result1, result2);
    assert_eq!(result1, Cow::Owned(21));
  }

  proptest! {
    #[test]
    fn prop_identity_law_str(s in "\\PC*") {
      let cow: Cow<'static, str> = Cow::Owned(s.clone());
      let id = |x: &str| x.to_string();
      let result = cow.clone().map(id);
      prop_assert_eq!(result, cow);
    }

    #[test]
    fn prop_composition_law_str(s in "\\PC*") {
      let cow: Cow<'static, str> = Cow::Owned(s);

      // Apply functions sequentially
      let result1 = cow.clone().map(to_uppercase).map(|s: &String| add_prefix(s.as_str()));

      // Apply composed function
      let result2 = cow.map(|s| add_prefix(&to_uppercase(s)));

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_identity_law_int(i in any::<i32>()) {
      let cow: Cow<'static, i32> = Cow::Owned(i);
      let id = |x: &i32| *x;
      let result = cow.clone().map(id);
      prop_assert_eq!(result, cow);
    }

    #[test]
    fn prop_composition_law_int(i in any::<i32>().prop_filter("Value too large", |&v| v > i32::MIN/2 && v < i32::MAX/2)) {
      let cow: Cow<'static, i32> = Cow::Owned(i);

      // Apply functions sequentially
      let result1 = cow.clone().map(double_int).map(add_one);

      // Apply composed function
      let result2 = cow.map(|i| add_one(&double_int(i)));

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }
  }
}
