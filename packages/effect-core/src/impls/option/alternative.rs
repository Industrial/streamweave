use crate::traits::alternative::Alternative;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Alternative<T> for Option<T> {
  fn empty() -> Self {
    None
  }

  fn alt(self, other: Self) -> Self {
    self.or(other)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions that are safe to use with property testing
  fn increment(x: &i32) -> i32 {
    x.saturating_add(1)
  }
  fn double(x: &i32) -> i32 {
    x.saturating_mul(2)
  }
  fn decrement(x: &i32) -> i32 {
    x.saturating_sub(1)
  }
  fn half(x: &i32) -> i32 {
    x / 2
  }
  fn square(x: &i32) -> i32 {
    x.saturating_mul(*x)
  }
  fn negate(x: &i32) -> i32 {
    x.checked_neg().unwrap_or(i32::MAX)
  }

  const FUNCTIONS: &[fn(&i32) -> i32] = &[increment, double, decrement, half, square, negate];

  proptest! {
    #[test]
    fn test_identity_law(x in any::<i32>()) {
      let empty = Option::<i32>::empty();
      assert_eq!(empty, None);

      let some_x = Some(x);
      // Identity law: alt(empty, x) == x
      prop_assert_eq!(empty.alt(some_x.clone()), some_x);
      // Identity law: alt(x, empty) == x
      prop_assert_eq!(some_x.clone().alt(empty), some_x);
    }

    #[test]
    fn test_associativity_law(
      x in any::<i32>(),
      y in any::<i32>(),
      z in any::<i32>()
    ) {
      let some_x = Some(x);
      let some_y = Some(y);
      let some_z = Some(z);

      // Associativity law: alt(alt(x, y), z) == alt(x, alt(y, z))
      let left = some_x.clone().alt(some_y.clone()).alt(some_z.clone());
      let right = some_x.alt(some_y.alt(some_z));
      prop_assert_eq!(left, right);
    }

    #[test]
    fn test_distributivity(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];

      let some_x = Some(x);
      let empty = Option::<i32>::empty();

      // Test that None.alt(Some(x)) is Some(x)
      prop_assert_eq!(empty.alt(some_x.clone()), some_x);

      // Test that Some(x).alt(None) is Some(x)
      prop_assert_eq!(some_x.clone().alt(empty), some_x);

      // Test that Some(x).alt(Some(y)) is Some(x)
      let some_fx = Some(f(&x));
      let some_gx = Some(g(&x));
      prop_assert_eq!(some_fx.clone().alt(some_gx.clone()), some_fx);
    }
  }
}
