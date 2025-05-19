use crate::traits::semigroup::Semigroup;
use crate::types::store::Store;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

// Store forms a semigroup when its value type A forms a semigroup
impl<S: CloneableThreadSafe, A: Semigroup> Semigroup for Store<S, A> {
  fn combine(self, other: Self) -> Self {
    // For Store, combine creates a new Store where:
    // - The position is taken from self
    // - The peek function combines the values from self and other at each position
    let self_peek = self.peek;
    let other_peek = other.peek;

    Store {
      peek: Arc::new(move |s: &S| {
        let a = (self_peek)(s);
        let b = (other_peek)(s);
        a.combine(b)
      }),
      pos: self.pos,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Remove the conflicting i32 implementation

  #[test]
  fn test_combine() {
    // Create two Stores with i32 values
    let store1 = Store::new(|x: &i32| x * 2, 5);
    let store2 = Store::new(|x: &i32| x + 3, 7);

    // Combine the stores
    let combined = store1.combine(store2);

    // At position 5 (from store1), we get (5*2) + (5+3) = 10 + 8 = 18
    assert_eq!(combined.extract(), 18);

    // At position 10, we get (10*2) + (10+3) = 20 + 13 = 33
    assert_eq!(combined.peek_at(&10), 33);
  }

  // Test associativity law with concrete values
  #[test]
  fn test_associativity_concrete() {
    let a = Store::new(|x: &i32| x * 2, 5);
    let b = Store::new(|x: &i32| x + 3, 7);
    let c = Store::new(|x: &i32| x - 1, 3);

    // Instead of using test_associativity helper, manually verify the associativity law:
    // (a + b) + c = a + (b + c)
    let left = a.clone().combine(b.clone()).combine(c.clone());
    let right = a.combine(b.combine(c));

    // Test at position 5 (from a)
    assert_eq!(left.extract(), right.extract());

    // Test at a few other positions
    assert_eq!(left.peek_at(&10), right.peek_at(&10));
    assert_eq!(left.peek_at(&20), right.peek_at(&20));
  }

  // Property-based tests
  proptest! {
      #[test]
      fn prop_combine_extract(pos1 in 1..100i32, pos2 in 1..100i32) {
          // Create two Stores with different functions and positions
          let store1 = Store::new(|x: &i32| x * 2, pos1);
          let store2 = Store::new(|x: &i32| x + 3, pos2);

          // Combine the stores
          let combined = store1.clone().combine(store2.clone());

          // The extract value should be the combination of the two extract values
          let expected = store1.extract().combine(store2.peek_at(&pos1));
          prop_assert_eq!(combined.extract(), expected);
      }

      #[test]
      fn prop_combine_peek(pos in 1..100i32, peek_pos in 1..100i32) {
          // Create two Stores with different functions but the same position
          let store1 = Store::new(|x: &i32| x * 2, pos);
          let store2 = Store::new(|x: &i32| x + 3, pos);

          // Combine the stores
          let combined = store1.clone().combine(store2.clone());

          // The peeked value should be the combination of the two peeked values
          let expected = store1.peek_at(&peek_pos).combine(store2.peek_at(&peek_pos));
          prop_assert_eq!(combined.peek_at(&peek_pos), expected);
      }
  }
}
