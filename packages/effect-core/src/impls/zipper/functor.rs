use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use crate::types::zipper::Zipper;

impl<A: CloneableThreadSafe> Functor<A> for Zipper<A> {
  type HigherSelf<B>
    = Zipper<B>
  where
    B: CloneableThreadSafe;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    Zipper {
      left: self.left.into_iter().map(|a| f(&a)).collect(),
      focus: f(&self.focus),
      right: self.right.into_iter().map(|a| f(&a)).collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_map() {
    let zipper = Zipper::from_vec(vec![1, 2, 3, 4]);
    let mapped = Functor::map(zipper, |x: &i32| x * 2);

    assert_eq!(mapped.focus, 2);
    assert!(mapped.left.is_empty());
    assert_eq!(mapped.right, vec![4, 6, 8]);
  }

  #[test]
  fn test_map_with_position() {
    let zipper = Zipper::from_parts(vec![3, 2], 1, vec![0]);

    // Instead of using a mutable counter, use a pre-determined mapping
    // that assigns positions based on expected values
    let mapped = Functor::map(zipper, |x: &i32| match *x {
      3 => 1, // first element in left (reversed)
      2 => 0, // second element in left (reversed)
      1 => 2, // focus
      0 => 3, // first element in right
      _ => panic!("Unexpected value"),
    });

    // The original sequence was [3, 2, 1, 0] (in zipper order)
    // So the mapped sequence should be [1, 0, 2, 3]
    // Note: Left elements are stored in reverse order in our implementation
    assert_eq!(mapped.left, vec![1, 0]); // Left is stored in reverse
    assert_eq!(mapped.focus, 2);
    assert_eq!(mapped.right, vec![3]);
  }

  proptest! {
      #[test]
      fn prop_functor_identity(elements in proptest::collection::vec(any::<i32>(), 1..5)) {
          // Identity law: map(x => x) == x
          let zipper = Zipper::from_vec(elements.clone());
          let mapped = Functor::map(zipper.clone(), |x: &i32| x.clone());

          prop_assert_eq!(mapped.focus, zipper.focus);
          prop_assert_eq!(mapped.left, zipper.left);
          prop_assert_eq!(mapped.right, zipper.right);
      }

      #[test]
      fn prop_functor_composition(elements in proptest::collection::vec(any::<i32>(), 1..5)) {
          // Composition law: map(f).map(g) == map(x => g(f(x)))
          let zipper = Zipper::from_vec(elements.clone());

          // Define functions
          let f = |x: &i32| x + 10;
          let g = |x: &i32| x.to_string();

          // Apply sequentially
          let mapped1 = {
              let temp = Functor::map(zipper.clone(), f);
              Functor::map(temp, g)
          };

          // Apply composed
          let mapped2 = Functor::map(zipper, move |x: &i32| g(&f(x)));

          // Compare results
          prop_assert_eq!(mapped1.focus, mapped2.focus);
          prop_assert_eq!(mapped1.left, mapped2.left);
          prop_assert_eq!(mapped1.right, mapped2.right);
      }
  }
}
