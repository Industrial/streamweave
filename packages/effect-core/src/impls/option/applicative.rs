use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Applicative<A> for Option<A> {
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    Some(value)
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    match (self, f) {
      (Some(a), Some(mut f)) => Some(f(&a)),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Identity law: pure(id) <*> v = v
  #[test]
  fn test_identity_law() {
    let value = Some(42i64);
    let id_fn = Option::<i64>::pure(|x: &i64| *x);
    let result = value.clone().ap(id_fn);
    assert_eq!(result, value);

    // Test with None
    let none: Option<i64> = None;
    let id_fn = Option::<i64>::pure(|x: &i64| *x);
    let result = none.clone().ap(id_fn);
    assert_eq!(result, none);
  }

  // Homomorphism law: pure(f) <*> pure(x) = pure(f(x))
  #[test]
  fn test_homomorphism_law() {
    let x = 42i64;
    let f = |x: &i64| x.to_string();

    let left = Option::<i64>::pure(x).ap(Option::<i64>::pure(f));
    let right = Option::<String>::pure(f(&x));

    assert_eq!(left, right);
  }

  // Type conversion tests
  #[test]
  fn test_type_conversions() {
    let value = Some(42i64);
    let f = Some(|x: &i64| x.to_string());
    let result = value.ap(f);
    assert_eq!(result, Some("42".to_string()));

    // Multiple type conversions
    let value = Some("hello");
    let f = Some(|s: &&str| s.len());
    let result = value.ap(f);
    assert_eq!(result, Some(5));

    // Struct creation
    #[derive(Debug, PartialEq, Clone)]
    struct Person {
      name: String,
      age: i64,
    }

    let name = Some("Alice".to_string());
    let age = Some(30i64);

    // Use owned values instead of references to avoid lifetime issues
    let create_person = |name: String| move |age: i64| Person { name, age };

    // Need to map the values to apply the function to owned values
    let person_fn = name.clone().map(create_person);
    let person = age.clone().and_then(|a| person_fn.map(|mut f| f(a)));

    assert_eq!(
      person,
      Some(Person {
        name: "Alice".to_string(),
        age: 30
      })
    );
  }

  proptest! {
    // Property-based test for identity law
    #[test]
    fn test_identity_law_prop(x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)) {
      let value = Some(x);
      let id_fn = Option::<i64>::pure(|x: &i64| *x);
      let result = value.clone().ap(id_fn);
      prop_assert_eq!(result, value);
    }

    // Property-based test for homomorphism law
    #[test]
    fn test_homomorphism_law_prop(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];

      let left = Option::<i64>::pure(x).ap(Option::<i64>::pure(f));
      let right = Option::<i64>::pure(f(&x));

      prop_assert_eq!(left, right);
    }
  }
}
