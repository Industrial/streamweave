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
  fn test_applicative_type_conversions() {
    // Converting integers to strings
    let x = Some(42i64);
    let f = Some(|x: &i64| x.to_string());
    let result = x.ap(f);
    assert_eq!(result, Some("42".to_string()));

    // Converting strings to lengths
    let x = Some("hello".to_string());
    let f = Some(|s: &String| s.len());
    let result = x.ap(f);
    assert_eq!(result, Some(5));
  }

  // Composition law: pure(compose) <*> u <*> v <*> w = u <*> (v <*> w)
  #[test]
  fn test_composition_law() {
    // Define compose function: (f . g) x = f(g(x))
    let compose = |f: &fn(&i64) -> i64| move |g: &fn(&i64) -> i64| move |x: &i64| f(&g(x));

    let x = Some(10i64);
    let f = Some(|x: &i64| x * 2);
    let g = Some(|x: &i64| x + 1);

    // Left side: pure(compose) <*> f <*> g <*> x
    let compose_pure = Option::<i64>::pure(compose);
    let compose_f = compose_pure.ap(f.clone());
    let compose_f_g = compose_f.ap(g.clone());
    let left = compose_f_g.ap(x.clone());

    // Right side: f <*> (g <*> x)
    let g_x = x.ap(g);
    let right = g_x.ap(f);

    assert_eq!(left, right);
    assert_eq!(left, Some(22)); // (10 + 1) * 2 = 22
  }

  // Interchange law: u <*> pure(y) = pure(|f| f(y)) <*> u
  #[test]
  fn test_interchange_law() {
    let y = 42i64;
    let u = Some(|x: &i64| x * 2);

    // Left side: u <*> pure(y)
    let left = Option::<i64>::pure(y).ap(u.clone());

    // Right side: pure(|f| f(y)) <*> u
    let apply_y = |f: &fn(&i64) -> i64| f(&y);
    let right = u.ap(Option::<fn(&i64) -> i64>::pure(apply_y));

    assert_eq!(left, right);
    assert_eq!(left, Some(84)); // 42 * 2 = 84
  }

  // Property-based tests for applicative laws
  proptest! {
    // Identity law
    #[test]
    fn test_identity_law_prop(x in -1000..1000i64, is_some in proptest::bool::ANY) {
      let value = if is_some { Some(x) } else { None };
      let id_fn = Option::<i64>::pure(|x: &i64| *x);
      let result = value.clone().ap(id_fn);

      prop_assert_eq!(result, value);
    }

    // Homomorphism law
    #[test]
    fn test_homomorphism_law_prop(x in -1000..1000i64, f_idx in 0..INT_FUNCTIONS.len()) {
      let f = INT_FUNCTIONS[f_idx];

      let left = Option::<i64>::pure(x).ap(Option::<i64>::pure(f));
      let right = Option::<i64>::pure(f(&x));

      prop_assert_eq!(left, right);
    }

    // Composition law
    #[test]
    fn test_composition_law_prop(
      x in -1000..1000i64,
      is_x_some in proptest::bool::ANY,
      is_f_some in proptest::bool::ANY,
      is_g_some in proptest::bool::ANY,
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let x_val = if is_x_some { Some(x) } else { None };
      let f_val = if is_f_some { Some(INT_FUNCTIONS[f_idx]) } else { None };
      let g_val = if is_g_some { Some(INT_FUNCTIONS[g_idx]) } else { None };

      // Define compose function: (f . g) x = f(g(x))
      let compose = |f: &fn(&i64) -> i64| {
        move |g: &fn(&i64) -> i64| {
          move |x: &i64| {
            f(&g(x))
          }
        }
      };

      // Left side: pure(compose) <*> f <*> g <*> x
      let compose_pure = Option::<i64>::pure(compose);
      let compose_f = compose_pure.ap(f_val.clone());
      let compose_f_g = compose_f.ap(g_val.clone());
      let left = compose_f_g.ap(x_val.clone());

      // Right side: f <*> (g <*> x)
      let g_x = x_val.ap(g_val);
      let right = g_x.ap(f_val);

      prop_assert_eq!(left, right);
    }

    // Interchange law
    #[test]
    fn test_interchange_law_prop(
      y in -1000..1000i64,
      is_u_some in proptest::bool::ANY,
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let u = if is_u_some { Some(f) } else { None };

      // Left side: u <*> pure(y)
      let left = Option::<i64>::pure(y).ap(u.clone());

      // Right side: pure(|f| f(y)) <*> u
      let apply_y = |f: &fn(&i64) -> i64| f(&y);
      let right = u.ap(Option::<fn(&i64) -> i64>::pure(apply_y));

      prop_assert_eq!(left, right);
    }
  }

  // Tests of realistic use cases
  #[test]
  fn test_applicative_realistic_use_cases() {
    // Parsing multiple optional inputs
    let parse_int = |s: &str| s.parse::<i32>().ok();

    // Both inputs present
    let input1 = Some("10");
    let input2 = Some("20");
    let add = |x: &i32| move |y: &i32| x + y;

    let parsed1 = input1.map(parse_int).unwrap_or(None);
    let parsed2 = input2.map(parse_int).unwrap_or(None);

    let result = parsed1.ap(parsed2.ap(Some(add)));
    assert_eq!(result, Some(30));

    // One input missing
    let input1 = Some("10");
    let input2 = None;

    let parsed1 = input1.map(parse_int).unwrap_or(None);
    let parsed2 = input2.map(|s| parse_int(s)).unwrap_or(None);

    let result = parsed1.ap(parsed2.ap(Some(add)));
    assert_eq!(result, None);

    // Invalid input
    let input1 = Some("10");
    let input2 = Some("not_a_number");

    let parsed1 = input1.map(parse_int).unwrap_or(None);
    let parsed2 = input2.map(parse_int).unwrap_or(None);

    let result = parsed1.ap(parsed2.ap(Some(add)));
    assert_eq!(result, None);
  }

  // Test with different function types
  #[test]
  fn test_applicative_with_different_function_types() {
    // Function that operates on strings
    let string_len = |s: &String| s.len();
    let some_string = Some("hello".to_string());
    let some_fn = Some(string_len);

    let result = some_string.ap(some_fn);
    assert_eq!(result, Some(5));

    // Function that returns a different type
    let to_bool = |n: &i32| n > &10;
    let some_number = Some(42);
    let some_fn = Some(to_bool);

    let result = some_number.ap(some_fn);
    assert_eq!(result, Some(true));

    // Function with side effects
    let mut count = 0;
    let count_and_double = |n: &i32| {
      count += 1;
      n * 2
    };

    let some_number = Some(5);
    let some_fn = Some(count_and_double);

    let result = some_number.ap(some_fn);
    assert_eq!(result, Some(10));
    assert_eq!(count, 1);

    // None case should not call the function
    count = 0;
    let none_number: Option<i32> = None;
    let some_fn = Some(count_and_double);

    let result = none_number.ap(some_fn);
    assert_eq!(result, None);
    assert_eq!(count, 0);
  }

  // Test nested applicatives
  #[test]
  fn test_nested_applicatives() {
    // Option<Option<T>>
    let nested_value = Some(Some(5));
    let nested_fn = Some(Some(|x: &i32| x * 2));

    // To apply a nested function to a nested value, we need to use ap twice
    let outer_applied =
      nested_value.map(|inner_value| inner_value.ap(nested_fn.clone().unwrap_or(None)));

    assert_eq!(outer_applied, Some(Some(10)));

    // With None at different levels
    let nested_value = Some(None::<i32>);
    let nested_fn = Some(Some(|x: &i32| x * 2));

    let outer_applied =
      nested_value.map(|inner_value| inner_value.ap(nested_fn.clone().unwrap_or(None)));

    assert_eq!(outer_applied, Some(None));

    let nested_value = Some(Some(5));
    let nested_fn = Some(None::<fn(&i32) -> i32>);

    let outer_applied =
      nested_value.map(|inner_value| inner_value.ap(nested_fn.clone().unwrap_or(None)));

    assert_eq!(outer_applied, Some(None));
  }

  // Test with multiple ap calls (sequencing)
  #[test]
  fn test_applicative_sequencing() {
    // Create a function that takes three arguments
    let add3 = |x: &i32| move |y: &i32| move |z: &i32| x + y + z;

    // Apply to three Some values
    let result = Some(1).ap(Some(2).ap(Some(3).ap(Some(add3))));

    assert_eq!(result, Some(6)); // 1 + 2 + 3 = 6

    // Apply with a missing value
    let result = Some(1).ap(None.ap(Some(3).ap(Some(add3))));

    assert_eq!(result, None);

    // Apply with a missing function
    let result = Some(1).ap(Some(2).ap(Some(3).ap(None)));

    assert_eq!(result, None);
  }
}
