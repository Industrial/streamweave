use crate::traits::profunctor::Profunctor;
use crate::types::pair::Pair;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Profunctor<A, B> for Pair<A, B> {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = Pair<C, D>;

  fn dimap<C: CloneableThreadSafe, D: CloneableThreadSafe, F, G>(
    self,
    f: F,
    g: G,
  ) -> Self::HigherSelf<C, D>
  where
    F: Fn(C) -> A + CloneableThreadSafe,
    G: Fn(B) -> D + CloneableThreadSafe,
  {
    // For Profunctor, the dimap operation:
    // - Takes a function f: C -> A (contravariant on input)
    // - Takes a function g: B -> D (covariant on output)
    // - Returns a Pair<C, D>

    // The correct implementation for Pair is straightforward:
    // - For the forward direction: C -> D, we compose f, self.apply, and g
    // - For the backward direction: D -> C, we compose g^-1, self.unapply, and f^-1
    // But we don't have g^-1 and f^-1, so we have to work around this

    Pair::new(
      move |c: C| g(self.apply(f(c))),
      move |_: D| {
        panic!("Cannot implement backward direction for Profunctor without inverse functions")
      },
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_profunctor_dimap() {
    let pair = Pair::new(
      |x: i32| x * 2, // forward function
      |y: i32| y / 2, // backward function
    );

    let dimapped = Profunctor::dimap(
      pair,
      |x: i32| x + 1, // pre-process input
      |y: i32| y - 1, // post-process output
    );

    // Test the forward direction: g ∘ original ∘ f
    let input = 5;
    let result = dimapped.apply(input);
    // Expected: ((5+1)*2)-1 = 11
    assert_eq!(result, 11);

    // The backward operation will panic in this implementation
  }

  #[test]
  fn test_profunctor_lmap() {
    let pair = Pair::new(|x: i32| x * 2, |y: i32| y / 2);

    let lmapped = Profunctor::lmap(pair, |x: i32| x + 1);

    // Test that lmap applies the function to the input
    let input = 5;
    let result = lmapped.apply(input);
    // Expected: (5+1)*2 = 12
    assert_eq!(result, 12);
  }

  #[test]
  fn test_profunctor_rmap() {
    let pair = Pair::new(|x: i32| x * 2, |y: i32| y / 2);

    let rmapped = Profunctor::rmap(pair, |y: i32| y - 1);

    // Test that rmap applies the function to the output
    let input = 5;
    let result = rmapped.apply(input);
    // Expected: (5*2)-1 = 9
    assert_eq!(result, 9);
  }

  proptest! {
      #[test]
      fn test_profunctor_dimap_identity_law(
          x in -1000..1000i32
      ) {
          // Identity law: dimap(id, id) == id
          let pair = Pair::new(
              |a: i32| a.saturating_mul(2),
              |b: i32| b.saturating_div(2),
          );

          let identity = |x: i32| x;
          let dimapped = Profunctor::dimap(pair.clone(), identity, identity);

          // Check that the law holds
          prop_assert_eq!(dimapped.apply(x), pair.apply(x));
      }
  }
}
