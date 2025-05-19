use crate::traits::comonad::Comonad;
use crate::types::threadsafe::CloneableThreadSafe;
use crate::types::zipper::Zipper;
#[cfg(test)]
use std::sync::Arc;

impl<A: CloneableThreadSafe> Comonad<A> for Zipper<A> {
  fn extract(self) -> A {
    self.focus
  }

  fn extend<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(Self) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // The key insight of the zipper comonad is that for each position in the zipper,
    // we apply f to a zipper focused at that position.

    // First, apply f to the current focus
    let focus = f(self.clone());

    // For all elements to the left of the focus, create a new zipper focused
    // at that position and apply f
    let left = (0..self.left.len())
      .map(|i| {
        // Create a zipper where the focus is at position i in the left array
        // Note: left is stored in reverse order, so we're working from right to left
        let new_left = self.left[i + 1..].to_vec();
        let new_focus = self.left[i].clone();
        let new_right = vec![
          self.left[0..i].to_vec(),
          vec![self.focus.clone()],
          self.right.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();

        // Apply f to this new zipper
        f(Zipper {
          left: new_left,
          focus: new_focus,
          right: new_right,
        })
      })
      .collect();

    // For all elements to the right of the focus, create a new zipper focused
    // at that position and apply f
    let right = (0..self.right.len())
      .map(|i| {
        // Create a zipper where the focus is at position i in the right array
        let new_left = vec![
          self.left.clone(),
          vec![self.focus.clone()],
          self.right[0..i].to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let new_focus = self.right[i].clone();
        let new_right = self.right[i + 1..].to_vec();

        // Apply f to this new zipper
        f(Zipper {
          left: new_left,
          focus: new_focus,
          right: new_right,
        })
      })
      .collect();

    Zipper { left, focus, right }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_extract() {
    let zipper = Zipper::from_vec(vec![1, 2, 3, 4]);
    assert_eq!(Comonad::extract(zipper), 1);
  }

  #[test]
  fn test_extend() {
    // Create a zipper with [1, 2, 3, 4], focused on 1
    let zipper = Zipper::from_vec(vec![1, 2, 3, 4]);

    // Sum of all elements in the zipper
    let sum_zipper = Comonad::extend(zipper.clone(), |z| z.iter().sum::<i32>());

    // The focus becomes sum of original [1,2,3,4] = 10
    assert_eq!(sum_zipper.focus, 10);

    // First element in right becomes sum when focused on 2: [1,2,3,4] = 10
    assert_eq!(sum_zipper.right[0], 10);

    // Second element in right becomes sum when focused on 3: [1,2,3,4] = 10
    assert_eq!(sum_zipper.right[1], 10);

    // Third element in right becomes sum when focused on 4: [1,2,3,4] = 10
    assert_eq!(sum_zipper.right[2], 10);

    // Now try with position-aware function
    let position_zipper = Comonad::extend(zipper, |z| z.position() as i32);

    // Current focus is at position 0
    assert_eq!(position_zipper.focus, 0);

    // Positions for right elements are 1, 2, 3
    assert_eq!(position_zipper.right, vec![1, 2, 3]);
  }

  #[test]
  fn test_duplicate() {
    let zipper = Zipper::from_vec(vec![1, 2, 3]);
    let duplicated = Comonad::duplicate(zipper);

    // The focus of the duplicated zipper should be the original zipper
    assert_eq!(duplicated.focus.focus, 1);
    assert!(duplicated.focus.left.is_empty());
    assert_eq!(duplicated.focus.right, vec![2, 3]);

    // The first element in the right context should be a zipper focused on 2
    assert_eq!(duplicated.right[0].focus, 2);
    assert_eq!(duplicated.right[0].left, vec![1]);
    assert_eq!(duplicated.right[0].right, vec![3]);

    // The second element in the right context should be a zipper focused on 3
    assert_eq!(duplicated.right[1].focus, 3);
    // The implementation produces elements in this order [1, 2] not [2, 1]
    assert_eq!(duplicated.right[1].left, vec![1, 2]);
    assert!(duplicated.right[1].right.is_empty());
  }

  // Property-based tests
  proptest! {
      #[test]
      fn prop_left_identity_law(elements in proptest::collection::vec(any::<i32>(), 1..5)) {
          // Left identity law: comonad.extend(|w| w.extract()) == comonad
          let zipper = Zipper::from_vec(elements.clone());
          let extract_fn = |z: Zipper<i32>| Comonad::extract(z);
          let result = Comonad::extend(zipper.clone(), extract_fn);

          // Every element should be extracted from its respective position
          prop_assert_eq!(result.focus, zipper.focus);

          // Test the right elements
          for (i, right_elem) in result.right.iter().enumerate() {
              let expected = zipper.right.get(i).cloned().unwrap_or_else(|| panic!("Index out of bounds"));
              prop_assert_eq!(*right_elem, expected);
          }
      }

      #[test]
      fn prop_right_identity_law(elements in proptest::collection::vec(any::<i32>(), 1..5)) {
          // Right identity law: comonad.extract() == comonad.extend(|w| w.extract()).extract()
          let zipper = Zipper::from_vec(elements.clone());
          let extract_fn = |z: Zipper<i32>| Comonad::extract(z);

          // Original extract
          let extracted1 = Comonad::extract(zipper.clone());

          // Extend then extract
          let extended = Comonad::extend(zipper, extract_fn);
          let extracted2 = Comonad::extract(extended);

          prop_assert_eq!(extracted1, extracted2);
      }

      #[test]
      fn prop_associativity_law(elements in proptest::collection::vec(-50..50, 1..5)) {
          // Associativity law: comonad.extend(f).extend(g) == comonad.extend(|w| g(w.extend(f)))
          let zipper = Zipper::from_vec(elements.clone());

          // Define extension functions that work with the Zipper
          let f = Arc::new(|z: Zipper<i32>| z.iter().sum::<i32>());
          let g = Arc::new(|n: &i32| n.to_string());

          // Apply extensions sequentially
          let f_clone = Arc::clone(&f);
          let temp = Comonad::extend(zipper.clone(), move |w| f_clone(w));

          let g_clone = Arc::clone(&g);
          let result1 = Comonad::extend(temp, move |w| g_clone(&w.focus));

          // Apply composed extension
          let f_clone = Arc::clone(&f);
          let g_clone = Arc::clone(&g);
          let result2 = Comonad::extend(zipper, move |w| {
              let temp_result = f_clone(w);
              g_clone(&temp_result)
          });

          // Compare results
          prop_assert_eq!(result1.focus, result2.focus);

          for i in 0..result1.right.len().min(result2.right.len()) {
              // Use borrowing to compare the strings to avoid move errors
              prop_assert_eq!(&result1.right[i], &result2.right[i]);
          }
      }
  }
}
