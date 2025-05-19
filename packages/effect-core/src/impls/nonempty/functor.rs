use crate::traits::functor::Functor;
use crate::types::nonempty::NonEmpty;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Functor<A> for NonEmpty<A> {
  type HigherSelf<U: CloneableThreadSafe> = NonEmpty<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a A) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    let head = f(&self.head);
    let tail = self.tail.into_iter().map(|a| f(&a)).collect();
    NonEmpty { head, tail }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_map() {
    let ne = NonEmpty::from_parts(1, vec![2, 3]);
    let result = Functor::map(ne, |x| x * 2);

    assert_eq!(result.head, 2);
    assert_eq!(result.tail, vec![4, 6]);
  }

  #[test]
  fn test_map_empty_tail() {
    let ne = NonEmpty::new(42);
    let result = Functor::map(ne, |x| x.to_string());

    assert_eq!(result.head, "42");
    assert_eq!(result.tail.len(), 0);
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // Identity law: functor.map(|x| x) == functor
      let ne = NonEmpty::from_parts(head, tail.clone());
      let identity_mapped = Functor::map(ne.clone(), |x| x.clone());

      prop_assert_eq!(identity_mapped.head, ne.head);
      prop_assert_eq!(identity_mapped.tail, ne.tail);
    }

    #[test]
    fn prop_composition_law(
      head in any::<i32>(),
      tail in proptest::collection::vec(any::<i32>(), 0..10),
    ) {
      // Composition law: functor.map(|x| g(f(x))) == functor.map(f).map(g)
      let ne = NonEmpty::from_parts(head, tail.clone());
      let f = |x: &i32| x.wrapping_mul(2);
      let g = |x: &i32| x.to_string();

      // Apply f then g
      let result1 = Functor::map(ne.clone(), f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let result2 = Functor::map(ne, move |x| g(&f(x)));

      prop_assert_eq!(result1.head, result2.head);
      prop_assert_eq!(result1.tail, result2.tail);
    }
  }
}
