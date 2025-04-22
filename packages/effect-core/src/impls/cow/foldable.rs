use crate::impls::cow::functor::CowFunctor;
use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::borrow::Cow;
use std::borrow::ToOwned;

// Implement Foldable for CowFunctor, not directly for Cow
impl<T: CloneableThreadSafe + ?Sized> Foldable<T> for CowFunctor<T> {
  fn fold<A, F>(self, _init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    // This is a placeholder implementation since CowFunctor is just a proxy type
    // The actual implementation is in the extension trait
    _init
  }

  fn fold_right<A, F>(self, _init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // This is a placeholder implementation since CowFunctor is just a proxy type
    // The actual implementation is in the extension trait
    _init
  }

  fn reduce<F>(self, _f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // This is a placeholder implementation since CowFunctor is just a proxy type
    // The actual implementation is in the extension trait
    None
  }

  fn reduce_right<F>(self, _f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // This is a placeholder implementation since CowFunctor is just a proxy type
    // The actual implementation is in the extension trait
    None
  }
}

// Extension trait to make Cow foldable operations more ergonomic
pub trait CowFoldableExt<'a, B, T>
where
  B: ?Sized + ToOwned<Owned = T> + 'a,
  T: CloneableThreadSafe + AsRef<B>,
{
  fn fold<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: FnMut(A, &B) -> A + CloneableThreadSafe;

  fn fold_right<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: FnMut(&B, A) -> A + CloneableThreadSafe;

  fn reduce<F>(self, f: F) -> Option<T>
  where
    F: FnMut(&B, &B) -> T + CloneableThreadSafe;

  fn reduce_right<F>(self, f: F) -> Option<T>
  where
    F: FnMut(&B, &B) -> T + CloneableThreadSafe;
}

// Implement the extension trait for Cow
impl<'a, B, T> CowFoldableExt<'a, B, T> for Cow<'a, B>
where
  B: ?Sized + ToOwned<Owned = T> + 'a,
  T: CloneableThreadSafe + AsRef<B>,
{
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: FnMut(A, &B) -> A + CloneableThreadSafe,
  {
    match self {
      Cow::Borrowed(borrowed) => f(init, borrowed),
      Cow::Owned(owned) => f(init, owned.as_ref()),
    }
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: FnMut(&B, A) -> A + CloneableThreadSafe,
  {
    match self {
      Cow::Borrowed(borrowed) => f(borrowed, init),
      Cow::Owned(owned) => f(owned.as_ref(), init),
    }
  }

  fn reduce<F>(self, _f: F) -> Option<T>
  where
    F: FnMut(&B, &B) -> T + CloneableThreadSafe,
  {
    match self {
      Cow::Borrowed(borrowed) => Some(borrowed.to_owned()),
      Cow::Owned(owned) => Some(owned),
    }
  }

  fn reduce_right<F>(self, _f: F) -> Option<T>
  where
    F: FnMut(&B, &B) -> T + CloneableThreadSafe,
  {
    match self {
      Cow::Borrowed(borrowed) => Some(borrowed.to_owned()),
      Cow::Owned(owned) => Some(owned),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::impls::cow::foldable::CowFoldableExt;
  use std::borrow::Cow;

  #[test]
  fn test_cow_fold() {
    let cow_str: Cow<str> = Cow::Borrowed("hello");
    let result = cow_str.fold(0, |acc, s| acc + s.len());
    assert_eq!(result, 5);

    let cow_str: Cow<str> = Cow::Owned("world".to_string());
    let result = cow_str.fold(0, |acc, s| acc + s.len());
    assert_eq!(result, 5);
  }

  #[test]
  fn test_cow_fold_right() {
    let cow_str: Cow<str> = Cow::Borrowed("hello");
    let result = cow_str.fold_right(0, |s, acc| acc + s.len());
    assert_eq!(result, 5);

    let cow_str: Cow<str> = Cow::Owned("world".to_string());
    let result = cow_str.fold_right(0, |s, acc| acc + s.len());
    assert_eq!(result, 5);
  }

  #[test]
  fn test_cow_reduce() {
    let cow_str: Cow<str> = Cow::Borrowed("hello");
    let result = cow_str.reduce(|_, _| "unused".to_string());
    assert_eq!(result, Some("hello".to_string()));

    let test_string = "world".to_string();
    let cow_str: Cow<str> = Cow::Owned(test_string.clone());
    let result = cow_str.reduce(|_, _| "unused".to_string());
    assert_eq!(result, Some(test_string));
  }

  #[test]
  fn test_cow_reduce_right() {
    let cow_str: Cow<str> = Cow::Borrowed("hello");
    let result = cow_str.reduce_right(|_, _| "unused".to_string());
    assert_eq!(result, Some("hello".to_string()));

    let test_string = "world".to_string();
    let cow_str: Cow<str> = Cow::Owned(test_string.clone());
    let result = cow_str.reduce_right(|_, _| "unused".to_string());
    assert_eq!(result, Some(test_string));
  }

  #[test]
  fn test_cow_with_complex_types() {
    // Test with a more complex type
    let vec = vec![1, 2, 3];
    let cow_slice: Cow<[i32]> = Cow::Borrowed(&vec);

    let result = cow_slice.fold(0, |acc, slice| acc + slice.iter().sum::<i32>());
    assert_eq!(result, 6);

    let owned_vec = vec![4, 5, 6];
    let cow_owned: Cow<[i32]> = Cow::Owned(owned_vec);

    let result = cow_owned.fold(0, |acc, slice| acc + slice.iter().sum::<i32>());
    assert_eq!(result, 15);
  }
}
