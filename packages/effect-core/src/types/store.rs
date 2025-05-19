//! Provides the [`Store`] comonad for environment-based computations.
//!
//! The [`Store`] comonad represents a computation with a state/position/focus
//! and a function that can compute values based on that position.
//! It's particularly useful for applications like:
//! - Cellular automata
//! - Image processing
//! - Spreadsheet formulas
//! - Caching computations

use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

/// The Store comonad, consisting of a function from S to A and a position value of type S.
///
/// The Store comonad (also called the Co-State comonad or Pointer comonad) represents
/// a value focused at a specific position, with the ability to compute related values
/// at other positions.
///
/// # Type Parameters
///
/// * `S` - The position/state/index type
/// * `A` - The value type
#[derive(Clone)]
pub struct Store<S, A>
where
  S: CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  /// The function that computes a value of type A from a position of type S
  pub peek: Arc<dyn Fn(&S) -> A + Send + Sync + 'static>,

  /// The current position/focus
  pub pos: S,
}

impl<S, A> Store<S, A>
where
  S: CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  /// Creates a new Store with the given function and position.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that computes values of type A from positions of type S
  /// * `s` - The initial position
  ///
  /// # Returns
  ///
  /// A new Store containing the function and position
  pub fn new<F>(f: F, s: S) -> Self
  where
    F: Fn(&S) -> A + Send + Sync + 'static,
  {
    Store {
      peek: Arc::new(f),
      pos: s,
    }
  }

  /// Creates a new Store by applying a function to the current position.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that transforms the position
  ///
  /// # Returns
  ///
  /// A new Store with the transformed position and the same peek function
  pub fn at<F>(&self, f: F) -> Self
  where
    F: FnOnce(&S) -> S,
  {
    Store {
      peek: Arc::clone(&self.peek),
      pos: f(&self.pos),
    }
  }

  /// Sets the position to a new value.
  ///
  /// # Arguments
  ///
  /// * `s` - The new position
  ///
  /// # Returns
  ///
  /// A new Store with the updated position
  pub fn set_pos(&self, s: S) -> Self {
    Store {
      peek: Arc::clone(&self.peek),
      pos: s,
    }
  }

  /// Gets the current value at the focused position.
  ///
  /// # Returns
  ///
  /// The value at the current position
  pub fn extract(&self) -> A {
    (self.peek)(&self.pos)
  }

  /// Gets the value at a given position without changing the focus.
  ///
  /// # Arguments
  ///
  /// * `s` - The position to peek at
  ///
  /// # Returns
  ///
  /// The value at the specified position
  pub fn peek_at(&self, s: &S) -> A {
    (self.peek)(s)
  }

  /// Transforms the function in the store using a higher-order function.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that transforms the peek function
  ///
  /// # Returns
  ///
  /// A new Store with the transformed peek function
  pub fn map_peek<F, B>(&self, f: F) -> Store<S, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
    B: CloneableThreadSafe,
  {
    let original_peek = Arc::clone(&self.peek);
    Store {
      peek: Arc::new(move |s| f((original_peek)(s))),
      pos: self.pos.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_store_operations() {
    // Create a Store that doubles the input position
    let store = Store::new(|x: &i32| x * 2, 5);

    // Check extract
    assert_eq!(store.extract(), 10); // 5 * 2 = 10

    // Check peeking at different positions
    assert_eq!(store.peek_at(&3), 6); // 3 * 2 = 6
    assert_eq!(store.peek_at(&10), 20); // 10 * 2 = 20

    // Check at
    let moved = store.at(|x| x + 2);
    assert_eq!(moved.extract(), 14); // (5 + 2) * 2 = 14

    // Check set_pos
    let repositioned = store.set_pos(8);
    assert_eq!(repositioned.extract(), 16); // 8 * 2 = 16

    // Check map_peek
    let mapped = store.map_peek(|x| x.to_string());
    assert_eq!(mapped.extract(), "10"); // (5 * 2).to_string() = "10"
  }
}
