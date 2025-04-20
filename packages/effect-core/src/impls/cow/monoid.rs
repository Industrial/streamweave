use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::borrow::Cow;
use std::borrow::ToOwned;

// Implementation for Cow<'static, str>
impl Monoid for Cow<'static, str> {
  fn empty() -> Self {
    Cow::Borrowed("")
  }
}

// Generic implementation for Cow<'static, T> where T: Monoid
impl<T> Monoid for Cow<'static, T>
where
  T: Monoid + ToOwned,
  <T as ToOwned>::Owned: Monoid + CloneableThreadSafe,
{
  fn empty() -> Self {
    Cow::Owned(T::empty().to_owned())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    let empty_cow: Cow<'static, str> = Monoid::empty();
    assert_eq!(empty_cow, "");
  }

  #[test]
  fn test_left_identity() {
    let empty: Cow<'static, str> = Monoid::empty();
    let value = Cow::Borrowed("hello");

    assert_eq!(empty.combine(value.clone()), value);
  }

  #[test]
  fn test_right_identity() {
    let empty: Cow<'static, str> = Monoid::empty();
    let value = Cow::Borrowed("hello");

    assert_eq!(value.clone().combine(empty), value);
  }

  // Property-based tests
  proptest! {
      #[test]
      fn prop_monoid_laws(s in "\\PC*") {
          // Create a Cow from the string
          let value: Cow<'static, str> = Cow::Owned(s);
          let empty: Cow<'static, str> = Monoid::empty();

          // Left identity: empty.combine(x) == x
          assert_eq!(empty.clone().combine(value.clone()), value.clone());

          // Right identity: x.combine(empty) == x
          assert_eq!(value.clone().combine(empty), value);
      }
  }
}
