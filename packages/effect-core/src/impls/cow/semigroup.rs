use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::borrow::Cow;
use std::borrow::ToOwned;

// Implementation for Cow<'static, str>
impl Semigroup for Cow<'static, str> {
  fn combine(self, other: Self) -> Self {
    // String has a Semigroup implementation that we can use
    match (self, other) {
      (Cow::Borrowed(a), Cow::Borrowed(b)) => {
        let mut result = String::with_capacity(a.len() + b.len());
        result.push_str(a);
        result.push_str(b);
        Cow::Owned(result)
      }
      (Cow::Borrowed(a), Cow::Owned(b)) => {
        let mut result = String::with_capacity(a.len() + b.len());
        result.push_str(a);
        result.push_str(&b);
        Cow::Owned(result)
      }
      (Cow::Owned(mut a), Cow::Borrowed(b)) => {
        a.push_str(b);
        Cow::Owned(a)
      }
      (Cow::Owned(mut a), Cow::Owned(b)) => {
        a.push_str(&b);
        Cow::Owned(a)
      }
    }
  }
}

// Generic implementation for Cow<'static, T> where T: Semigroup
impl<T> Semigroup for Cow<'static, T>
where
  T: Semigroup + ToOwned,
  <T as ToOwned>::Owned: Semigroup + CloneableThreadSafe,
{
  fn combine(self, other: Self) -> Self {
    match (self, other) {
      (Cow::Borrowed(a), Cow::Borrowed(b)) => {
        // Convert borrowed values to owned values to combine them
        let owned_a = a.to_owned();
        let owned_b = b.to_owned();
        Cow::Owned(owned_a.combine(owned_b))
      }
      (Cow::Borrowed(a), Cow::Owned(b)) => {
        let owned_a = a.to_owned();
        Cow::Owned(owned_a.combine(b))
      }
      (Cow::Owned(a), Cow::Borrowed(b)) => {
        let owned_b = b.to_owned();
        Cow::Owned(a.combine(owned_b))
      }
      (Cow::Owned(a), Cow::Owned(b)) => Cow::Owned(a.combine(b)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_combine_borrowed() {
    let a: Cow<'static, str> = Cow::Borrowed("Hello, ");
    let b: Cow<'static, str> = Cow::Borrowed("World!");
    let c = a.combine(b);
    assert_eq!(c, "Hello, World!");
  }

  #[test]
  fn test_combine_owned() {
    let a: Cow<'static, str> = Cow::Owned("Hello, ".to_string());
    let b: Cow<'static, str> = Cow::Owned("World!".to_string());
    let c = a.combine(b);
    assert_eq!(c, "Hello, World!");
  }

  #[test]
  fn test_combine_mixed() {
    let a: Cow<'static, str> = Cow::Borrowed("Hello, ");
    let b: Cow<'static, str> = Cow::Owned("World!".to_string());
    let c = a.combine(b);
    assert_eq!(c, "Hello, World!");
  }

  // Property-based tests
  proptest! {
      #[test]
      fn prop_associativity(a in "\\PC*", b in "\\PC*", c in "\\PC*") {
          let cow_a: Cow<'static, str> = Cow::Owned(a);
          let cow_b: Cow<'static, str> = Cow::Owned(b);
          let cow_c: Cow<'static, str> = Cow::Owned(c);

          let r1 = cow_a.clone().combine(cow_b.clone()).combine(cow_c.clone());
          let r2 = cow_a.clone().combine(cow_b.clone().combine(cow_c.clone()));

          assert_eq!(r1, r2);
      }
  }
}
