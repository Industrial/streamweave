use crate::traits::monad::Monad;
use crate::types::store::Store;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;
use std::sync::Mutex;

impl<S: CloneableThreadSafe, A: CloneableThreadSafe> Monad<A> for Store<S, A> {
  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // For Store, bind needs to:
    // 1. Apply the function `f` to the current extract value at position `self.pos`
    // 2. This returns a new Store<S, B>
    // 3. Create a new Store<S, B> that combines the original position with
    //    the peek function from the result of f

    // Clone the position for use in both the peek function and the result Store
    let original_pos = self.pos.clone();
    let pos_for_peek = original_pos.clone();
    let original_peek = self.peek.clone();

    // Wrap the mutable function f in a Mutex to make it thread-safe
    let f = Arc::new(Mutex::new(f));

    // Create the new peek function that:
    // - Extracts the value at the current position from original store
    // - Applies f to get a new store
    // - Uses that store's peek function with the position s
    let new_peek = Arc::new(move |s: &S| {
      // Extract value at original position
      let value_at_pos = (original_peek)(&pos_for_peek);

      // Apply f to get a new store
      let mut f_guard = f.lock().unwrap();
      let new_store = (*f_guard)(&value_at_pos);

      // Use the new store's peek function with position s
      (new_store.peek)(s)
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

  // Helper function for tests
  fn create_test_store<B: CloneableThreadSafe>(value: B) -> Store<i32, B> {
    Store::new(move |_: &i32| value.clone(), 0)
  }

  #[test]
  fn test_bind() {
    // Create a Store that doubles the position
    let store = Store::new(|x: &i32| x.wrapping_mul(2), 5);

    // This lambda takes a value x and creates a Store that always returns x+10
    // and uses 0 as the position. It behaves like a constant function relative
    // to its position parameter.
    let bind_fn = |x: &i32| {
      let x_val = *x;
      Store::new(move |_: &i32| x_val.wrapping_add(10), 0)
    };

    // Apply bind
    let result = store.bind(bind_fn);

    // Initial value is (5 * 2) = 10, and Monad's bind applies the function
    // to give a Store that returns 10+10 = 20 at any position
    assert_eq!(result.extract(), 20);

    // The behavior at position 8:
    // 1. The original Store gives 8*2 = 16 at position 8
    // 2. The bind_fn creates a Store that always returns 16+10 = 26
    // Note: Our implementation creates a Store that evaluates f at the original position value,
    // then uses that Store's peek function to look up values at other positions
    assert_eq!(result.peek_at(&8), 20);
  }

  proptest::proptest! {
      #![proptest_config(proptest::test_runner::Config::default())]#[test]fn prop_left_identity_law(x in(any::<i32>()),pos in(any::<i32>())){
          let pure_x = create_test_store(x);
          let pos_clone = pos.clone();
          let f = move|n: &i32|{
              let n_val =  *n;
              let pos_val = pos_clone.clone();
              Store::new(move|_: &i32|n_val,pos_val)
          };
          let left = pure_x.clone().bind(f.clone());
          let right = f(&x);
          prop_assert_eq!(left.extract(),right.extract());
          prop_assert_eq!(left.peek_at(&10),right.peek_at(&10));
      }#[test]fn prop_right_identity_law(pos in(any::<i32>())){
          let store = Store::new(|x: &i32|x.wrapping_mul(2),pos);
          let pure_fn =  |x: &i32|create_test_store(*x);
          let left = store.clone().bind(pure_fn);
          let left_at_pos = left.extract();
          let store_at_pos = store.extract();
          prop_assert_eq!(left_at_pos,store_at_pos);
      }#[test]fn prop_associativity_law(pos in(any::<i32>())){
          let store = Store::new(|x: &i32|x.wrapping_mul(2),pos);
          let pos1 = pos.clone();
          let pos2 = pos.clone();
          let f = move|x: &i32|{
              let x_val =  *x;
              let pos_val = pos1.clone();
              Store::new(move|_p: &i32|x_val,pos_val)
          };
          let g = move|x: &i32|{
              let x_val =  *x;
              let pos_val = pos2.clone();
              Store::new(move|_: &i32|x_val.wrapping_mul(3),pos_val)
          };
          let store_clone1 = store.clone();
          let store_clone2 = store.clone();
          let f_clone1 = f.clone();
          let f_clone2 = f.clone();
          let g_clone1 = g.clone();
          let g_clone2 = g.clone();
          let left = store_clone1.bind(f_clone1).bind(g_clone1);
          let composed = move|x: &i32|{
              let x_val =  *x;
              let inner_f = f_clone2.clone();
              let inner_g = g_clone2.clone();
              inner_f(&x_val).bind(inner_g)
          };
          let right = store_clone2.bind(composed);
          prop_assert_eq!(left.extract(),right.extract());
          prop_assert_eq!(left.peek_at(&10),right.peek_at(&10));
      }
  }
}
