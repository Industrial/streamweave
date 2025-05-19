use crate::traits::comonad::Comonad;
use crate::types::store::Store;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<S: CloneableThreadSafe, A: CloneableThreadSafe> Comonad<A> for Store<S, A> {
  fn extract(self) -> A {
    (self.peek)(&self.pos)
  }

  fn extend<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(Self) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // For the store comonad, extending means creating a new Store where:
    // - The peek function creates a Store at the position being peeked and applies f
    // - The position stays the same

    // Clone the pieces we'll need for the new peek function
    let original_peek = Arc::clone(&self.peek);
    let original_pos = self.pos.clone();

    // We need to clone f to make it usable in the Fn closure required by Arc
    // First, we'll wrap it in an Arc+Mutex to make it safely shared
    let f = std::sync::Arc::new(std::sync::Mutex::new(f));

    // Now we can create a Fn closure that uses our wrapped FnMut
    let new_peek = Arc::new(move |s: &S| {
      let store = Store {
        peek: Arc::clone(&original_peek),
        pos: s.clone(),
      };
      // Get a mutable reference to f and apply it
      let mut guard = f.lock().unwrap();
      (*guard)(store)
    });

    Store {
      peek: new_peek,
      pos: original_pos,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_extract() {
    let store = Store::new(|x: &i32| x * 2, 5);
    assert_eq!(Comonad::extract(store), 10); // 5 * 2 = 10
  }

  #[test]
  fn test_extend() {
    // Store that doubles the position
    let store = Store::new(|x: &i32| x * 2, 5);

    // Extend with a function that adds 1 to the current position and extracts
    let extend_fn = |s: Store<i32, i32>| {
      let new_store = s.at(|x| x + 1);
      // Need to call extract directly to avoid ownership issues
      new_store.extract()
    };

    let result = Comonad::extend(store, extend_fn);

    // At position 5, we get (5+1)*2 = 12
    let extract_val = result.clone().extract();
    assert_eq!(extract_val, 12);

    // At position 10, we get (10+1)*2 = 22
    assert_eq!(result.peek_at(&10), 22);
  }

  #[test]
  fn test_duplicate() {
    let store = Store::new(|x: &i32| x * 2, 5);
    let duplicated = Comonad::duplicate(store);

    // The outer store contains the inner store at position 5
    let inner_store = duplicated.clone().extract();
    // Need to call extract directly to avoid ownership issues
    let inner_val = inner_store.extract();
    assert_eq!(inner_val, 10); // 5 * 2 = 10

    // We can also peek at other positions
    let inner_store_at_7 = duplicated.peek_at(&7);
    let inner_val_at_7 = inner_store_at_7.extract();
    assert_eq!(inner_val_at_7, 14); // 7 * 2 = 14
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_left_identity_law(pos in 1..100i32) {
      // Left identity law: comonad.extend(|w| w.extract()) == comonad
      let comonad = Store::new(|x: &i32| x * 2, pos);
      let comonad_clone = comonad.clone();

      // Create a function that extracts the value (can't use Comonad::extract directly due to ownership)
      let extract_fn = |w: Store<i32, i32>| w.extract();
      let result = Comonad::extend(comonad, extract_fn);

      // The behavior should be the same at the current position and other positions
      let result_val = result.clone().extract();
      let comonad_val = comonad_clone.clone().extract();
      prop_assert_eq!(result_val, comonad_val);

      prop_assert_eq!(result.peek_at(&(pos + 5)), comonad_clone.peek_at(&(pos + 5)));
      prop_assert_eq!(result.peek_at(&(pos - 1)), comonad_clone.peek_at(&(pos - 1)));
    }

    #[test]
    fn prop_right_identity_law(pos in 1..100i32) {
      // Right identity law: comonad.extract() == comonad.extend(|w| w.extract()).extract()
      let comonad = Store::new(|x: &i32| x * 2, pos);

      // Extract from the original comonad
      let extracted = comonad.clone().extract();

      // Define an extract function
      let extract_fn = |w: Store<i32, i32>| w.extract();

      // Apply the function via extend and then extract
      let extended = Comonad::extend(comonad, extract_fn);
      let result = extended.extract();

      prop_assert_eq!(extracted, result);
    }

    #[test]
    fn prop_associativity_law(pos in 1..100i32) {
      // Define functions using explicit moveability
      let f_fn = Arc::new(|w: Store<i32, i32>| w.extract() + 3);
      let g_fn = Arc::new(|x: i32| x.to_string());

      // Create initial comonad
      let comonad = Store::new(|x: &i32| x * 2, pos);

      // Apply extensions sequentially with cloned functions
      let f1 = Arc::clone(&f_fn);
      let result1 = Comonad::extend(comonad.clone(), move |w| f1(w));

      let g1 = Arc::clone(&g_fn);
      let result1 = Comonad::extend(result1, move |w| g1(w.extract()));

      // Apply composed extension with cloned functions
      let f2 = Arc::clone(&f_fn);
      let g2 = Arc::clone(&g_fn);
      let result2 = Comonad::extend(comonad, move |w| {
        g2(f2(w))
      });

      // Compare results
      let val1 = result1.clone().extract();
      let val2 = result2.clone().extract();
      prop_assert_eq!(val1, val2);

      // Check behavior at another position
      let peek1 = result1.peek_at(&(pos + 5));
      let peek2 = result2.peek_at(&(pos + 5));
      prop_assert_eq!(peek1, peek2);
    }
  }
}
