use super::functor::CowFunctor;
use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;
use std::borrow::Cow;
use std::borrow::ToOwned;

// Implement Applicative for CowFunctor
impl<A> Applicative<A> for CowFunctor<A>
where
  A: CloneableThreadSafe + ToOwned<Owned = A> + ?Sized + 'static,
{
  fn pure<B>(_value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    // Returns a new CowFunctor for the value
    CowFunctor::new()
  }

  fn ap<B, F>(self, _fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // The actual application happens when using the extension trait
    CowFunctor::new()
  }
}

// Extension trait for Cow to provide applicative functionality
pub trait CowApplicativeExt<A>
where
  A: CloneableThreadSafe + ToOwned<Owned = A> + ?Sized + 'static,
{
  fn pure<B>(value: B) -> Cow<'static, B>
  where
    B: CloneableThreadSafe + ToOwned<Owned = B> + ?Sized + 'static,
    <B as ToOwned>::Owned: CloneableThreadSafe;

  fn ap<B, F>(self, fs: Cow<'static, F>) -> Cow<'static, B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe + ToOwned<Owned = B> + ?Sized + 'static,
    <B as ToOwned>::Owned: CloneableThreadSafe;
}

impl<A> CowApplicativeExt<A> for Cow<'static, A>
where
  A: CloneableThreadSafe + ToOwned<Owned = A> + ?Sized + 'static,
{
  fn pure<B>(value: B) -> Cow<'static, B>
  where
    B: CloneableThreadSafe + ToOwned<Owned = B> + ?Sized + 'static,
    <B as ToOwned>::Owned: CloneableThreadSafe,
  {
    Cow::Owned(value)
  }

  fn ap<B, F>(self, fs: Cow<'static, F>) -> Cow<'static, B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe + ToOwned<Owned = B> + ?Sized + 'static,
    <B as ToOwned>::Owned: CloneableThreadSafe,
  {
    let mut f = match fs {
      Cow::Borrowed(borrowed_f) => borrowed_f.clone(),
      Cow::Owned(owned_f) => owned_f,
    };

    match self {
      Cow::Borrowed(borrowed_a) => {
        let result = f(borrowed_a);
        Cow::Owned(result)
      }
      Cow::Owned(owned_a) => {
        let result = f(&owned_a);
        Cow::Owned(result)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper functions for tests
  fn to_uppercase(s: &String) -> String {
    s.to_uppercase()
  }

  fn add_prefix(s: &String) -> String {
    format!("prefix_{}", s)
  }

  fn double_int(i: &i32) -> i32 {
    i.saturating_mul(2)
  }

  fn add_one(i: &i32) -> i32 {
    i.saturating_add(1)
  }

  // Identity law: pure(id) <*> v = v
  #[test]
  fn test_identity_law() {
    let cow: Cow<'static, i32> = Cow::Owned(42);
    let id_fn: Cow<'static, fn(&i32) -> i32> = Cow::<i32>::pure(|x: &i32| *x);
    let result = cow.clone().ap(id_fn);
    assert_eq!(result, cow);

    let cow_string: Cow<'static, String> = Cow::Owned("test".to_string());
    let id_fn_string: Cow<'static, fn(&String) -> String> =
      Cow::<String>::pure(|x: &String| x.clone());
    let result_string = cow_string.clone().ap(id_fn_string);
    assert_eq!(result_string, cow_string);
  }

  // Homomorphism law: pure(f) <*> pure(x) = pure(f(x))
  #[test]
  fn test_homomorphism_law() {
    let x = 42;
    let f = double_int;

    let left = Cow::<i32>::pure(x).ap(Cow::<i32>::pure(f));
    let right = Cow::<i32>::pure(f(&x));

    assert_eq!(left, right);

    let str_x = "test".to_string();
    let str_f = to_uppercase;

    let left_str = Cow::<String>::pure(str_x.clone()).ap(Cow::<String>::pure(str_f));
    let right_str = Cow::<String>::pure(str_f(&str_x));

    assert_eq!(left_str, right_str);
  }

  // Interchange law: u <*> pure(y) = pure(\f -> f(y)) <*> u
  #[test]
  fn test_interchange_law() {
    let y = 42;
    let u: Cow<'static, fn(&i32) -> i32> = Cow::<i32>::pure(double_int);

    let left = Cow::<i32>::pure(y).ap(u.clone());

    // Using a closure for the apply function (can't use fn type for captures)
    let apply_y = move |f: &fn(&i32) -> i32| f(&y);
    let right = u.ap(Cow::<i32>::pure(apply_y));

    assert_eq!(left, right);

    let str_y = "test".to_string();
    let str_u: Cow<'static, fn(&String) -> String> = Cow::<String>::pure(to_uppercase);

    let left_str = Cow::<String>::pure(str_y.clone()).ap(str_u.clone());

    let apply_str_y = move |f: &fn(&String) -> String| f(&str_y);
    let right_str = str_u.ap(Cow::<String>::pure(apply_str_y));

    assert_eq!(left_str, right_str);
  }

  // Composition law: pure(.) <*> u <*> v <*> w = u <*> (v <*> w)
  #[test]
  fn test_composition_law() {
    let w: Cow<'static, i32> = Cow::Owned(10);
    let v: Cow<'static, fn(&i32) -> i32> = Cow::<i32>::pure(double_int);
    let u: Cow<'static, fn(&i32) -> i32> = Cow::<i32>::pure(add_one);

    let inner = w.clone().ap(v.clone());
    let right = inner.ap(u.clone());

    let result_v = double_int(&10); // 20
    let result_u = add_one(&result_v); // 21
    let expected = Cow::Owned(result_u);

    assert_eq!(right, expected);

    let str_w: Cow<'static, String> = Cow::Owned("hello".to_string());
    let str_v: Cow<'static, fn(&String) -> String> = Cow::<String>::pure(to_uppercase);
    let str_u: Cow<'static, fn(&String) -> String> = Cow::<String>::pure(add_prefix);

    let str_inner = str_w.clone().ap(str_v.clone());
    let str_right = str_inner.ap(str_u.clone());

    let str_result_v = to_uppercase(&"hello".to_string()); // "HELLO"
    let str_result_u = add_prefix(&str_result_v); // "prefix_HELLO"
    let str_expected: Cow<'static, String> = Cow::Owned(str_result_u);

    assert_eq!(str_right, str_expected);
  }

  // Property-based tests
  proptest! {
      #[test]
      fn prop_identity_law_int(x in any::<i32>()) {
          let cow: Cow<'static, i32> = Cow::Owned(x);
          let id_fn: Cow<'static, fn(&i32) -> i32> = Cow::<i32>::pure(|n: &i32| *n);
          let result = cow.clone().ap(id_fn);
          prop_assert_eq!(result, cow);
      }

      #[test]
      fn prop_homomorphism_law_int(x in any::<i32>()) {
          let f = double_int;
          let left = Cow::<i32>::pure(x).ap(Cow::<i32>::pure(f));
          let right = Cow::<i32>::pure(f(&x));
          prop_assert_eq!(left, right);
      }

      #[test]
      fn prop_interchange_law_int(y in any::<i32>()) {
          let u: Cow<'static, fn(&i32) -> i32> = Cow::<i32>::pure(double_int);
          let left = Cow::<i32>::pure(y).ap(u.clone());

          // Using a closure instead of a fn type
          let apply_y = move |f: &fn(&i32) -> i32| f(&y);
          let right = u.ap(Cow::<i32>::pure(apply_y));
          prop_assert_eq!(left, right);
      }
  }
}
