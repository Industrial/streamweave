use crate::traits::comonad::Comonad;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Comonad<A> for Box<A> {
  fn extract(self) -> A {
    *self
  }

  fn extend<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(Self) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    Box::new(f(self))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_extract() {
    let boxed = Box::new(42);
    assert_eq!(Comonad::extract(boxed), 42);
  }

  #[test]
  fn test_extend() {
    let boxed = Box::new(42);
    let result = Comonad::extend(boxed, |b| *b * 2);
    assert_eq!(*result, 84);
  }

  #[test]
  fn test_duplicate() {
    let boxed = Box::new(42);
    let duplicated = Comonad::duplicate(boxed);
    assert_eq!(Comonad::extract(Comonad::extract(duplicated)), 42);
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_left_identity_law(x in any::<i32>()) {
      // Left identity law: comonad.extend(|w| w.extract()) == comonad
      let comonad = Box::new(x);
      let result = Comonad::extend(comonad.clone(), |w| Comonad::extract(w));

      prop_assert_eq!(*result, x);
    }

    #[test]
    fn prop_right_identity_law(x in any::<i32>()) {
      // Right identity law: comonad.extract() == comonad.extend(|w| w.extract()).extract()
      let comonad = Box::new(x);
      let extracted = Comonad::extract(comonad.clone());
      let extended = Comonad::extend(comonad, |w| Comonad::extract(w));
      let result = Comonad::extract(extended);

      prop_assert_eq!(extracted, result);
    }

    #[test]
    fn prop_associativity_law(x in any::<i32>()) {
      // Associativity law: comonad.extend(f).extend(g) == comonad.extend(|w| g(w.extend(f)))
      let comonad = Box::new(x);

      // Define extension functions as Fn closures to avoid borrowing issues
      // Use wrapping operations to avoid integer overflow
      let f = move |w: Box<i32>| (*w).wrapping_mul(2);
      let g = move |w: Box<i32>| (*w).wrapping_add(10);

      // Apply extensions sequentially
      let result1 = Comonad::extend(comonad.clone(), f);
      let result1 = Comonad::extend(result1, g);

      // Apply composed extension
      // Use move keyword to ensure f and g are captured by value rather than by reference
      let result2 = Comonad::extend(comonad, move |w| {
        // Since f and g are already Fn closures (not FnMut or FnOnce),
        // we can call them directly without wrapping in Box::new
        g(Box::new(f(w)))
      });

      prop_assert_eq!(*result1, *result2);
    }
  }
}
