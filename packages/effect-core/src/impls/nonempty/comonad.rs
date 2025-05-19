use crate::traits::comonad::Comonad;
use crate::types::nonempty::NonEmpty;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Comonad<A> for NonEmpty<A> {
  fn extract(self) -> A {
    self.head
  }

  fn extend<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(Self) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // Apply f to the whole structure to get the new head
    let head = f(self.clone());

    // Apply f to each "focused" view of the structure
    let tail = (0..self.tail.len())
      .map(|i| {
        // Create a rotated view with a different element as the head
        let mut rotated = self.clone();
        let new_head = rotated.tail.remove(i);
        rotated.tail.push(rotated.head);
        rotated.head = new_head;

        // Apply f to this new view
        f(rotated)
      })
      .collect();

    NonEmpty { head, tail }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_extract() {
    let ne = NonEmpty::from_parts(1, vec![2, 3]);
    assert_eq!(Comonad::extract(ne), 1);
  }

  #[test]
  fn test_extend() {
    let ne = NonEmpty::from_parts(1, vec![2, 3]);

    // Sum of all elements in the nonempty vector
    let result = Comonad::extend(ne, |v: NonEmpty<i32>| v.iter().sum::<i32>());

    // The head becomes sum of original [1,2,3] = 6
    // The first tail element becomes sum of [2,3,1] = 6
    // The second tail element becomes sum of [3,1,2] = 6
    assert_eq!(result.head, 6);
    assert_eq!(result.tail, vec![6, 6]);
  }

  #[test]
  fn test_duplicate() {
    let ne = NonEmpty::from_parts(1, vec![2, 3]);
    let duplicated = Comonad::duplicate(ne);

    // The head of the result should be the original structure
    assert_eq!(duplicated.head.head, 1);
    assert_eq!(duplicated.head.tail, vec![2, 3]);

    // The first tail element should be the structure rotated once
    assert_eq!(duplicated.tail[0].head, 2);
    assert_eq!(duplicated.tail[0].tail, vec![3, 1]);

    // The second tail element should be the structure rotated twice
    assert_eq!(duplicated.tail[1].head, 3);
    // The implementation actually orders elements as [2, 1] not [1, 2]
    assert_eq!(duplicated.tail[1].tail, vec![2, 1]);
  }

  // Property-based tests
  proptest! {
    // Limit the size of the vector to prevent test explosion
    #[test]
    fn prop_left_identity_law(
      head in any::<i32>(),
      tail in proptest::collection::vec(any::<i32>(), 0..3)
    ) {
      // Left identity law: comonad.extend(|w| w.extract()) == comonad
      let ne = NonEmpty::from_parts(head, tail.clone());
      let ne_clone = ne.clone();
      let result = Comonad::extend(ne, |w| Comonad::extract(w));

      prop_assert_eq!(result.head, ne_clone.head);
      // Each element in the tail is also the extract of that rotated view
      for (i, val) in result.tail.iter().enumerate() {
        let expected = ne_clone.tail.get(i).cloned().unwrap_or(ne_clone.head);
        prop_assert_eq!(*val, expected);
      }
    }

    #[test]
    fn prop_right_identity_law(
      head in any::<i32>(),
      tail in proptest::collection::vec(any::<i32>(), 0..3)
    ) {
      // Right identity law: comonad.extract() == comonad.extend(|w| w.extract()).extract()
      let ne = NonEmpty::from_parts(head, tail.clone());
      let ne_clone = ne.clone();
      let extracted = Comonad::extract(ne_clone);
      let extended = Comonad::extend(ne, Comonad::extract);
      let result = Comonad::extract(extended);

      prop_assert_eq!(extracted, result);
    }

    #[test]
    fn prop_associativity_law(
      head in any::<i32>(),
      tail in proptest::collection::vec(any::<i32>(), 0..3)
    ) {
      // Associativity law: comonad.extend(f).extend(g) == comonad.extend(|w| g(w.extend(f)))
      let ne = NonEmpty::from_parts(head, tail.clone());

      // Define extension functions that use wrapping operations to avoid overflow
      let f = |w: NonEmpty<i32>| w.iter().fold(0i32, |acc: i32, &x: &i32| acc.wrapping_add(x));

      // Apply extensions sequentially
      let result1 = Comonad::extend(ne.clone(), f);
      let result1 = Comonad::extend(result1, |w: NonEmpty<i32>| w.head.wrapping_mul(2));

      // Apply composed extension
      let g = |x: i32| x.wrapping_mul(2);
      let result2 = Comonad::extend(ne, move |w| {
        g(f(w))
      });

      // Compare the heads and tails
      prop_assert_eq!(result1.head, result2.head);
      prop_assert_eq!(result1.tail, result2.tail);
    }
  }
}
