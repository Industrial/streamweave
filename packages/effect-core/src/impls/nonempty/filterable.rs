// Filterable implementation for NonEmpty
use crate::traits::filterable::Filterable;
use crate::types::nonempty::NonEmpty;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Filterable<A> for NonEmpty<A> {
  // We use Option<NonEmpty<B>> as the filtered type to handle the possibility of all elements being filtered out
  type Filtered<B: CloneableThreadSafe> = Option<NonEmpty<B>>;

  fn filter_map<B, F>(self, mut f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // Apply the function to the head
    match f(&self.head) {
      Some(head_mapped) => {
        // Head is retained, create a new NonEmpty<B>
        // Filter and map the tail elements
        let tail_mapped: Vec<B> = self.tail.iter().filter_map(|a| f(a)).collect();

        // Return a NonEmpty with the mapped head and filtered tail
        Some(NonEmpty {
          head: head_mapped,
          tail: tail_mapped,
        })
      }
      None => {
        // Head is filtered out, try to find a new head from the tail
        let filtered: Vec<B> = self.tail.iter().filter_map(|a| f(a)).collect();

        if filtered.is_empty() {
          // All elements filtered out
          None
        } else {
          // Create a new NonEmpty from the filtered elements
          let head = filtered[0].clone();
          let tail = filtered[1..].to_vec();

          Some(NonEmpty { head, tail })
        }
      }
    }
  }

  // Default implementation uses filter_map
  fn filter<F>(self, predicate: F) -> Self::Filtered<A>
  where
    F: for<'a> FnMut(&'a A) -> bool + CloneableThreadSafe,
    A: Clone,
  {
    let mut predicate = predicate;
    self.filter_map(move |x| if predicate(x) { Some(x.clone()) } else { None })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::functor::Functor;
  use proptest::prelude::*;

  #[test]
  fn test_filter_map_keep_all() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);

    // Map all elements to their string representations
    let result = Filterable::filter_map(ne, |x| Some(x.to_string()));

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.head, "1");
    assert_eq!(result.tail, vec!["2", "3", "4"]);
  }

  #[test]
  fn test_filter_map_filter_some() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4, 5]);

    // Keep only even numbers, convert to strings
    let result = Filterable::filter_map(ne, |x| {
      if x % 2 == 0 {
        Some(x.to_string())
      } else {
        None
      }
    });

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.head, "2");
    assert_eq!(result.tail, vec!["4"]);
  }

  #[test]
  fn test_filter_map_filter_all() {
    let ne = NonEmpty::from_parts(1, vec![3, 5, 7]);

    // Try to keep only even numbers (none present)
    let result = Filterable::filter_map(ne, |x| if x % 2 == 0 { Some(x * 2) } else { None });

    // Should return None since all elements are filtered out
    assert!(result.is_none());
  }

  #[test]
  fn test_filter_keep_all() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);

    // Keep all elements
    let result = Filterable::filter(ne, |_| true);

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.head, 1);
    assert_eq!(result.tail, vec![2, 3, 4]);
  }

  #[test]
  fn test_filter_keep_some() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4, 5]);

    // Keep only even numbers
    let result = Filterable::filter(ne, |x| x % 2 == 0);

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.head, 2);
    assert_eq!(result.tail, vec![4]);
  }

  #[test]
  fn test_filter_keep_none() {
    let ne = NonEmpty::from_parts(1, vec![3, 5, 7]);

    // Try to keep only even numbers (none present)
    let result = Filterable::filter(ne, |x| x % 2 == 0);

    // Should return None since all elements are filtered out
    assert!(result.is_none());
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_preservation(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // Identity preservation: filterable.filter_map(|x| Some(x)) == filterable
      let ne = NonEmpty::from_parts(head, tail.clone());
      let result = Filterable::filter_map(ne.clone(), |x| Some(*x));

      prop_assert!(result.is_some());
      let result = result.unwrap();
      prop_assert_eq!(result.head, ne.head);
      prop_assert_eq!(result.tail, ne.tail);
    }

    #[test]
    fn prop_annihilation(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // Annihilation: filterable.filter_map(|_| None) == empty (None)
      let ne = NonEmpty::from_parts(head, tail);
      let result = Filterable::filter_map::<(), _>(ne, |_| None);

      prop_assert!(result.is_none());
    }

    #[test]
    fn prop_consistency_with_functor(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // Consistency with Functor: filterable.filter_map(|x| Some(f(x))) == filterable.map(f)
      let ne = NonEmpty::from_parts(head, tail.clone());

      // Define the function outside the closure
      // Use wrapping_mul to avoid overflow
      let double = |x: &i32| x.wrapping_mul(2);

      let result1 = Filterable::filter_map(ne.clone(), move |x| Some(double(x)));
      let result2 = Functor::map(ne, double);

      prop_assert!(result1.is_some());
      let result1 = result1.unwrap();
      prop_assert_eq!(result1.head, result2.head);
      prop_assert_eq!(result1.tail, result2.tail);
    }

    #[test]
    fn prop_distributivity(head in any::<i32>(), tail in proptest::collection::vec(any::<i32>(), 0..10)) {
      // Simplified distributivity test: filtering by even then odd should result in None
      let ne = NonEmpty::from_parts(head, tail);

      let f = |x: &i32| if x % 2 == 0 { Some(*x) } else { None };
      let g = |x: &i32| if x % 2 != 0 { Some(*x) } else { None };

      // filter_map(f).filter_map(g) should be empty since no number is both even and odd
      let result = Filterable::filter_map(ne, f)
        .and_then(|filtered| Filterable::filter_map(filtered, g));

      prop_assert!(result.is_none());
    }
  }
}
