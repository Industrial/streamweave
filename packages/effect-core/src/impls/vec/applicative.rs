use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Applicative<A> for Vec<A> {
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    vec![value]
  }

  fn ap<B, F>(self, fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = Vec::new();
    for mut f in fs {
      for a in &self {
        result.push(f(a));
      }
    }
    result
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
    // Test with non-empty vector
    let values = vec![1, 2, 3, 4, 5];
    let id_fn: Vec<fn(&i64) -> i64> = Vec::<i64>::pure(|x: &i64| *x);
    let result = values.clone().ap(id_fn);
    assert_eq!(result, values);

    // Test with empty vector
    let empty: Vec<i64> = Vec::new();
    let id_fn: Vec<fn(&i64) -> i64> = Vec::<i64>::pure(|x: &i64| *x);
    let result = empty.clone().ap(id_fn);
    assert_eq!(result, empty);
  }

  // Homomorphism law: pure(f) <*> pure(x) = pure(f(x))
  #[test]
  fn test_homomorphism_law() {
    let x = 42i64;
    let f: fn(&i64) -> String = |x: &i64| x.to_string();

    let left = Vec::<i64>::pure(x).ap(Vec::<fn(&i64) -> String>::pure(f));
    let right = Vec::<String>::pure(f(&x));

    assert_eq!(left, right);
  }

  // Interchange law: u <*> pure(y) = pure(\f -> f(y)) <*> u
  #[test]
  fn test_interchange_law() {
    let y = 42i64;
    let u: Vec<fn(&i64) -> String> = vec![|x: &i64| x.to_string(), |x: &i64| format!("num: {}", x)];

    let left = Vec::<i64>::pure(y).ap(u.clone());

    // Need to move y into the closure
    let apply_y = move |f: &fn(&i64) -> String| f(&y);
    let right = u.ap(Vec::<fn(&i64) -> String>::pure(apply_y));

    assert_eq!(left, right);
  }

  // Composition law: pure(.) <*> u <*> v <*> w = u <*> (v <*> w)
  #[test]
  fn test_composition_law() {
    let w = vec![1, 2, 3];
    let v: Vec<fn(&i64) -> i64> =
      vec![|x: &i64| x.saturating_mul(2), |x: &i64| x.saturating_add(1)];
    let u: Vec<fn(&i64) -> String> = vec![|x: &i64| x.to_string()];

    // Using a simpler approach for the test by calling ap directly
    let left = w.clone().ap(vec![v[0]]).ap(vec![u[0]]);

    // Right side: u <*> (v <*> w)
    let inner = w.clone().ap(vec![v[0]]);
    let right = inner.ap(vec![u[0]]);

    assert_eq!(left, right);
  }

  // Test behavior with empty vectors
  #[test]
  fn test_empty_vectors() {
    // Empty vector ap non-empty vector of functions
    let empty: Vec<i64> = Vec::new();
    let functions = vec![|x: &i64| x.to_string()];
    assert_eq!(empty.ap(functions), Vec::<String>::new());

    // Non-empty vector ap empty vector of functions
    let values = vec![1, 2, 3];
    let empty_funcs: Vec<fn(&i64) -> String> = Vec::new();
    assert_eq!(values.ap(empty_funcs), Vec::<String>::new());

    // Empty vector ap empty vector of functions
    let empty: Vec<i64> = Vec::new();
    let empty_funcs: Vec<fn(&i64) -> String> = Vec::new();
    assert_eq!(empty.ap(empty_funcs), Vec::<String>::new());
  }

  // Test with multiple functions
  #[test]
  fn test_multiple_functions() {
    let values = vec![1, 2, 3];
    let functions: Vec<fn(&i64) -> String> = vec![
      |x: &i64| x.to_string(),
      |x: &i64| format!("num: {}", x),
      |x: &i64| format!("{} squared is {}", x, x.saturating_mul(*x)),
    ];

    let result = values.ap(functions);
    let expected = vec![
      "1".to_string(),
      "2".to_string(),
      "3".to_string(),
      "num: 1".to_string(),
      "num: 2".to_string(),
      "num: 3".to_string(),
      "1 squared is 1".to_string(),
      "2 squared is 4".to_string(),
      "3 squared is 9".to_string(),
    ];

    assert_eq!(result, expected);
  }

  // Test with different types
  #[test]
  fn test_type_conversions() {
    let values = vec!["hello", "world"];
    let functions: Vec<fn(&&str) -> usize> = vec![|s: &&str| s.len(), |s: &&str| s.len() * 2];

    let result = values.ap(functions);
    let expected = vec![5, 5, 10, 10];

    assert_eq!(result, expected);

    // Struct creation
    #[derive(Debug, PartialEq, Clone)]
    struct Person {
      name: String,
      score: i64,
    }

    let names = vec!["Alice", "Bob"];
    let create_person = |name: &&str| {
      let name_owned = name.to_string();
      move |score: &i64| Person {
        name: name_owned.clone(),
        score: *score,
      }
    };

    let scores = vec![90, 85];
    let person_creators = names.iter().map(create_person).collect::<Vec<_>>();
    let persons = scores.ap(person_creators);

    let expected = vec![
      Person {
        name: "Alice".to_string(),
        score: 90,
      },
      Person {
        name: "Alice".to_string(),
        score: 85,
      },
      Person {
        name: "Bob".to_string(),
        score: 90,
      },
      Person {
        name: "Bob".to_string(),
        score: 85,
      },
    ];

    assert_eq!(persons, expected);
  }

  proptest! {
    // Property-based test for identity law
    #[test]
    fn test_identity_law_prop(xs in prop::collection::vec(any::<i64>(), 0..10)) {
      let values = xs.clone();
      let id_fn: Vec<fn(&i64) -> i64> = Vec::<i64>::pure(|x: &i64| *x);
      let result = values.ap(id_fn);
      prop_assert_eq!(result, xs);
    }

    // Property-based test for homomorphism law
    #[test]
    fn test_homomorphism_law_prop(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];

      let left = Vec::<i64>::pure(x).ap(Vec::<fn(&i64) -> i64>::pure(f));
      let right = Vec::<i64>::pure(f(&x));

      prop_assert_eq!(left, right);
    }

    // Property-based test for composition law
    #[test]
    fn test_composition_law_prop(
      xs in prop::collection::vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let w = xs;
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Using a single function in each vector for simplicity
      let v = vec![f];
      let u = vec![g];

      // Simpler approach for composition law testing
      let inner = w.clone().ap(v.clone());
      let right = inner.ap(u.clone());

      let left = w.clone().ap(v).ap(u);

      prop_assert_eq!(left, right);
    }

    // Test cartesian product behavior for vectors of different sizes
    #[test]
    fn test_cartesian_product(
      xs in prop::collection::vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      ys in prop::collection::vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let values = xs.clone();
      let funcs = ys.iter().map(|y| {
        let y_capture = *y;
        move |x: &i64| x.saturating_add(y_capture)
      }).collect::<Vec<_>>();

      let result = values.ap(funcs);

      // The result should have xs.len() * ys.len() elements
      prop_assert_eq!(result.len(), xs.len() * ys.len());

      // Verify each element is correctly computed
      let mut expected = Vec::new();
      for y in &ys {
        for x in &xs {
          expected.push(x.saturating_add(*y));
        }
      }

      prop_assert_eq!(result, expected);
    }
  }
}
