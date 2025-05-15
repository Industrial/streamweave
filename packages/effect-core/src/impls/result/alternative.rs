use crate::traits::alternative::Alternative;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Alternative<T> for Result<T, E> {
  fn empty() -> Self {
    // Use a default error value - in practice, users should handle this properly
    Err(unsafe { std::mem::zeroed() })
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

  // Helper function to create Results with a defined error type
  fn result_empty<T: CloneableThreadSafe, E: CloneableThreadSafe + Default>() -> Result<T, E> {
    Err(E::default())
  }

  proptest! {
    #[test]
    fn test_identity_law(x in any::<i32>()) {
      let empty: Result<i32, ()> = result_empty();

      let ok_x = Ok(x);
      // Identity law: alt(empty, x) == x
      prop_assert_eq!(empty.clone().alt(ok_x.clone()), ok_x);
      // Identity law: alt(x, empty) == x
      prop_assert_eq!(ok_x.clone().alt(empty), ok_x);
    }

    #[test]
    fn test_associativity_law(
      x in any::<i32>(),
      y in any::<i32>(),
      z in any::<i32>()
    ) {
      let ok_x = Ok(x);
      let ok_y = Ok(y);
      let ok_z = Ok(z);

      // Associativity law: alt(alt(x, y), z) == alt(x, alt(y, z))
      let left: Result<i32, ()> = ok_x.clone().alt(ok_y.clone()).alt(ok_z.clone());
      let right: Result<i32, ()> = ok_x.alt(ok_y.alt(ok_z));
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

      let ok_x = Ok(x);
      let empty: Result<i32, ()> = result_empty();

      // Test that Err.alt(Ok(x)) is Ok(x)
      prop_assert_eq!(empty.clone().alt(ok_x.clone()), ok_x);

      // Test that Ok(x).alt(Err) is Ok(x)
      prop_assert_eq!(ok_x.clone().alt(empty), ok_x);

      // Test that Ok(x).alt(Ok(y)) is Ok(x)
      let ok_fx: Result<i32, ()> = Ok(f(&x));
      let ok_gx: Result<i32, ()> = Ok(g(&x));
      prop_assert_eq!(ok_fx.clone().alt(ok_gx.clone()), ok_fx);
    }
  }
}
