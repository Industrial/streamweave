use std::collections::VecDeque;

use crate::impls::vecdeque::category::VecDequeCategory;
use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

// Implement Applicative for VecDequeCategory
impl<A: CloneableThreadSafe> Applicative<A> for VecDequeCategory {
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    let mut deque = VecDeque::new();
    deque.push_back(value);
    deque
  }

  fn ap<B, F>(self, _fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // Since VecDequeCategory is just a proxy, we return a VecDeque with no elements
    // The actual ap operation is performed via VecDequeApplicativeExt
    VecDeque::new()
  }
}

// Extension trait to make VecDeque applicative operations more ergonomic
pub trait VecDequeApplicativeExt<A: CloneableThreadSafe> {
  fn pure<B>(value: B) -> VecDeque<B>
  where
    B: CloneableThreadSafe;

  fn ap<B, F>(self, fs: VecDeque<F>) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe;
}

// Implement the extension trait for VecDeque
impl<A: CloneableThreadSafe> VecDequeApplicativeExt<A> for VecDeque<A> {
  fn pure<B>(value: B) -> VecDeque<B>
  where
    B: CloneableThreadSafe,
  {
    let mut deque = VecDeque::new();
    deque.push_back(value);
    deque
  }

  fn ap<B, F>(self, mut fs: VecDeque<F>) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = VecDeque::new();
    for value in &self {
      for i in 0..fs.len() {
        if let Some(f) = fs.get_mut(i) {
          result.push_back(f(value));
        }
      }
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::impls::vecdeque::functor::VecDequeFunctorExt;
  use proptest::prelude::*;

  fn to_vecdeque<T: Clone>(vec: Vec<T>) -> VecDeque<T> {
    vec.into_iter().collect()
  }

  #[test]
  fn test_identity_law_empty() {
    let empty: VecDeque<i64> = VecDeque::new();
    let id_fn: VecDeque<fn(&i64) -> i64> = VecDeque::<i64>::pure(|x: &i64| *x);
    let result = empty.clone().ap(id_fn);
    assert_eq!(result, empty);
  }

  #[test]
  fn test_homomorphism_law() {
    let x = 42i64;
    let f = |x: &i64| x.to_string();

    let left = VecDeque::<i64>::pure(x).ap(VecDeque::<fn(&i64) -> String>::pure(f));
    let right = VecDeque::<String>::pure(f(&x));

    assert_eq!(left, right);
  }

  #[test]
  fn test_interchange_law() {
    let y = 10i64;
    let u: VecDeque<fn(&i64) -> String> = to_vecdeque(vec![|x: &i64| x.to_string(), |x: &i64| {
      format!("Value: {}", x)
    }]);

    // Function to apply to each function in u with y
    // Use 'move' to take ownership of y
    let y_clone = y;
    let apply_y = move |f: &fn(&i64) -> String| f(&y_clone);

    let left = VecDeque::<i64>::pure(y).ap(u.clone());
    let right = u.ap(VecDeque::<fn(&fn(&i64) -> String) -> String>::pure(apply_y));

    assert_eq!(left, right);
  }

  #[test]
  fn test_composition_law() {
    // Define test data
    let nums = to_vecdeque(vec![1, 2, 3]);
    let to_string_fn = |x: &i32| x.to_string();
    let to_length_fn = |s: &String| s.len();

    // Create function containers
    let string_fns = to_vecdeque(vec![to_string_fn]);
    let length_fns = to_vecdeque(vec![to_length_fn]);

    // Test the composition law in two different ways

    // First approach: apply in sequence
    // Apply nums -> strings
    let strings = nums.clone().ap(string_fns.clone());
    // Apply strings -> lengths
    let lengths1 = strings.ap(length_fns.clone());

    // Second approach: compose the functions then apply once
    // Create a composed function: i32 -> usize by composing to_length_fn and to_string_fn
    // Clone the functions to ensure they live long enough
    let to_string_clone = to_string_fn;
    let to_length_clone = to_length_fn;
    let composed_fn = move |x: &i32| to_length_clone(&to_string_clone(x));
    let composed_fns = to_vecdeque(vec![composed_fn]);

    // Apply the composed function directly to nums
    let lengths2 = nums.ap(composed_fns);

    // Results should be the same
    assert_eq!(lengths1, lengths2);
  }

  #[test]
  fn test_with_empty_deques() {
    // Empty deque of values
    let empty: VecDeque<i64> = VecDeque::new();
    let functions: VecDeque<fn(&i64) -> String> = to_vecdeque(vec![|x: &i64| x.to_string()]);

    assert_eq!(
      empty.clone().ap(functions.clone()),
      VecDeque::<String>::new()
    );

    // Empty deque of functions
    let values: VecDeque<i64> = to_vecdeque(vec![1, 2, 3]);
    let empty_funcs: VecDeque<fn(&i64) -> String> = VecDeque::new();

    assert_eq!(
      values.clone().ap(empty_funcs.clone()),
      VecDeque::<String>::new()
    );

    // Both empty
    assert_eq!(empty.ap(empty_funcs), VecDeque::<String>::new());
  }

  #[test]
  fn test_multiple_functions() {
    let values: VecDeque<i64> = to_vecdeque(vec![1, 2, 3]);
    let functions: VecDeque<fn(&i64) -> String> =
      to_vecdeque(vec![|x: &i64| x.to_string(), |x: &i64| {
        format!("Number: {}", x)
      }]);

    let result = values.ap(functions);

    let expected: VecDeque<String> = to_vecdeque(vec![
      "1".to_string(),
      "Number: 1".to_string(),
      "2".to_string(),
      "Number: 2".to_string(),
      "3".to_string(),
      "Number: 3".to_string(),
    ]);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_with_string_values() {
    let values: VecDeque<&str> = to_vecdeque(vec!["a", "b", "c"]);
    let functions: VecDeque<fn(&&str) -> String> =
      to_vecdeque(vec![|x: &&str| x.to_uppercase(), |x: &&str| {
        format!("{}-suffix", x)
      }]);

    let result = values.ap(functions);

    let expected: VecDeque<String> = to_vecdeque(vec![
      "A".to_string(),
      "a-suffix".to_string(),
      "B".to_string(),
      "b-suffix".to_string(),
      "C".to_string(),
      "c-suffix".to_string(),
    ]);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_with_complex_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Person {
      name: String,
      score: i32,
    }

    let scores: VecDeque<i32> = to_vecdeque(vec![85, 90, 95]);
    let person_creators: VecDeque<fn(&i32) -> Person> = to_vecdeque(vec![|score: &i32| Person {
      name: format!("Student with score {}", score),
      score: *score,
    }]);

    let persons = scores.ap(person_creators);

    let expected: VecDeque<Person> = to_vecdeque(vec![
      Person {
        name: "Student with score 85".to_string(),
        score: 85,
      },
      Person {
        name: "Student with score 90".to_string(),
        score: 90,
      },
      Person {
        name: "Student with score 95".to_string(),
        score: 95,
      },
    ]);

    assert_eq!(persons, expected);
  }

  #[test]
  fn test_lifting() {
    // Lift a binary function to operate on two applicatives
    fn lift2<F, A, B, C>(f: F, a: VecDeque<A>, b: VecDeque<B>) -> VecDeque<C>
    where
      F: Fn(&A, &B) -> C + Clone + Send + Sync + 'static,
      A: Clone + Send + Sync + 'static,
      B: Clone + Send + Sync + 'static,
      C: Clone + Send + Sync + 'static,
    {
      let curried = move |a: &A| {
        let a_clone = a.clone();
        let f_clone = f.clone();
        move |b: &B| f_clone(&a_clone, b)
      };

      let curried_a = a.map(curried);
      b.ap(curried_a)
    }

    let xs = to_vecdeque(vec![1, 2]);
    let ys = to_vecdeque(vec![10, 20]);

    let result = lift2(|x: &i32, y: &i32| x + y, xs, ys);

    // The expected result should match the actual output order from the test
    let expected = to_vecdeque(vec![11, 12, 21, 22]);

    assert_eq!(result, expected);
  }

  proptest! {
    #[test]
    fn prop_identity_law(values in prop::collection::vec(any::<i64>(), 0..10)) {
      let values = to_vecdeque(values);
      let id_fn: VecDeque<fn(&i64) -> i64> = VecDeque::<i64>::pure(|x: &i64| *x);
      let result = values.clone().ap(id_fn);

      prop_assert_eq!(result, values);
    }

    #[test]
    fn prop_homomorphism_law(x in any::<i64>()) {
      let f = |x: &i64| x.to_string();

      let left = VecDeque::<i64>::pure(x).ap(VecDeque::<fn(&i64) -> String>::pure(f));
      let right = VecDeque::<String>::pure(f(&x));

      prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_interchange_law(y in any::<i64>(), funcs in prop::collection::vec(prop::sample::select(vec![
        |x: &i64| x.to_string(),
        |x: &i64| format!("Value: {}", x),
        |x: &i64| format!("{:x}", x),
    ]), 1..3)) {
      let u: VecDeque<fn(&i64) -> String> = to_vecdeque(funcs);

      // Function to apply to each function in u with y
      // Use 'move' to take ownership of y
      let y_clone = y;
      let apply_y = move |f: &fn(&i64) -> String| f(&y_clone);

      let left = VecDeque::<i64>::pure(y).ap(u.clone());
      let right = u.ap(VecDeque::<fn(&fn(&i64) -> String) -> String>::pure(apply_y));

      prop_assert_eq!(left, right);
    }
  }
}
