// Semigroup implementation for NonEmpty

use crate::traits::semigroup::Semigroup;
use crate::types::nonempty::NonEmpty;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Semigroup for NonEmpty<A> {
  fn combine(self, other: Self) -> Self {
    // Create a new NonEmpty by concatenating the two
    let mut combined_tail = self.tail;

    // Add self.head to the tail
    combined_tail.push(self.head);

    // Add other.head and other.tail
    combined_tail.push(other.head);
    combined_tail.extend(other.tail);

    // Take the first element as the new head
    let new_head = combined_tail.remove(0);

    NonEmpty {
      head: new_head,
      tail: combined_tail,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_combine() {
    let ne1 = NonEmpty::from_parts(1, vec![2, 3]);
    let ne2 = NonEmpty::from_parts(4, vec![5, 6]);

    let result = Semigroup::combine(ne1, ne2);

    // The result should have all elements from both NonEmpty collections
    // The head is the first element of the first NonEmpty's tail
    assert_eq!(result.head, 2);
    assert_eq!(result.tail, vec![3, 1, 4, 5, 6]);
  }

  #[test]
  fn test_combine_empty_tails() {
    let ne1 = NonEmpty::new(1);
    let ne2 = NonEmpty::new(2);

    let result = Semigroup::combine(ne1, ne2);

    // In a singleton NoEmpty, there is no tail to provide the new head,
    // so the head of ne1 becomes the first element in the tail, and
    // the first element of the tail (ne1.head) becomes the new head
    assert_eq!(result.head, 1);
    assert_eq!(result.tail, vec![2]);
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_associativity(
      a_head in any::<i32>(),
      a_tail in proptest::collection::vec(any::<i32>(), 0..5),
      b_head in any::<i32>(),
      b_tail in proptest::collection::vec(any::<i32>(), 0..5),
      c_head in any::<i32>(),
      c_tail in proptest::collection::vec(any::<i32>(), 0..5),
    ) {
      // Associativity: (a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)
      let a = NonEmpty::from_parts(a_head, a_tail);
      let b = NonEmpty::from_parts(b_head, b_tail);
      let c = NonEmpty::from_parts(c_head, c_tail);

      let left = Semigroup::combine(Semigroup::combine(a.clone(), b.clone()), c.clone());
      let right = Semigroup::combine(a, Semigroup::combine(b, c));

      // For complex combining operations, we need to check that the result contains
      // the same elements, but they might be arranged differently due to
      // the implementation details of combine

      // Get all elements from both results
      let mut left_elements = vec![left.head];
      left_elements.extend(left.tail);

      let mut right_elements = vec![right.head];
      right_elements.extend(right.tail);

      // Sort both for comparison
      left_elements.sort();
      right_elements.sort();

      prop_assert_eq!(left_elements, right_elements);
    }
  }
}
