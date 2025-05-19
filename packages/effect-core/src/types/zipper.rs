//! Provides the [`Zipper`] type for focused traversal and manipulation of sequential data.
//!
//! The [`Zipper`] is a data structure that represents a collection with a cursor or focus
//! on a specific element, along with the context (elements to the left and right).
//! It allows for efficient traversal and updates while maintaining immutability.

use crate::types::threadsafe::CloneableThreadSafe;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// A zipper data structure for efficient traversal and manipulation of sequential data.
///
/// The Zipper represents a collection by splitting it into three parts:
/// - A focused element in the center
/// - A list of elements to the left (stored in reverse order for efficiency)
/// - A list of elements to the right
///
/// This structure enables O(1) operations for moving focus left/right and updating elements.
///
/// # Type Parameters
///
/// * `T` - The type of elements stored in the zipper
#[derive(Clone, PartialEq, Eq)]
pub struct Zipper<T: CloneableThreadSafe> {
  /// Elements to the left of the focus, stored in reverse order
  pub left: Vec<T>,
  /// The focused element
  pub focus: T,
  /// Elements to the right of the focus
  pub right: Vec<T>,
}

impl<T: CloneableThreadSafe> Zipper<T> {
  /// Creates a new Zipper with the specified focus and empty left and right contexts.
  ///
  /// # Arguments
  ///
  /// * `focus` - The element to focus on
  ///
  /// # Returns
  ///
  /// A new Zipper with the focused element and empty contexts
  pub fn new(focus: T) -> Self {
    Zipper {
      left: Vec::new(),
      focus,
      right: Vec::new(),
    }
  }

  /// Creates a new Zipper from a vector, focusing on the first element.
  ///
  /// # Arguments
  ///
  /// * `vec` - A vector with at least one element
  ///
  /// # Returns
  ///
  /// A new Zipper focused on the first element of the vector
  ///
  /// # Panics
  ///
  /// Panics if the provided vector is empty.
  pub fn from_vec(mut vec: Vec<T>) -> Self {
    if vec.is_empty() {
      panic!("Cannot create Zipper from an empty vector");
    }

    let focus = vec.remove(0);
    Zipper {
      left: Vec::new(),
      focus,
      right: vec,
    }
  }

  /// Attempts to create a new Zipper from a vector, focusing on the first element.
  ///
  /// # Arguments
  ///
  /// * `vec` - A vector that may be empty
  ///
  /// # Returns
  ///
  /// `Some(Zipper)` if the vector had at least one element, `None` otherwise
  pub fn from_vec_safe(mut vec: Vec<T>) -> Option<Self> {
    if vec.is_empty() {
      None
    } else {
      let focus = vec.remove(0);
      Some(Zipper {
        left: Vec::new(),
        focus,
        right: vec,
      })
    }
  }

  /// Creates a new Zipper with the given left context, focus, and right context.
  ///
  /// # Arguments
  ///
  /// * `left` - Elements to the left of the focus (in reverse order)
  /// * `focus` - The focused element
  /// * `right` - Elements to the right of the focus
  ///
  /// # Returns
  ///
  /// A new Zipper with the specified focus and contexts
  pub fn from_parts(left: Vec<T>, focus: T, right: Vec<T>) -> Self {
    Zipper { left, focus, right }
  }

  /// Moves the focus one position to the left.
  ///
  /// # Returns
  ///
  /// A new Zipper with the focus moved left, or `None` if already at the leftmost position
  pub fn move_left(&self) -> Option<Self> {
    if self.left.is_empty() {
      None
    } else {
      let mut left = self.left.clone();
      let new_focus = left.pop().unwrap();
      let mut right = self.right.clone();
      right.insert(0, self.focus.clone());

      Some(Zipper {
        left,
        focus: new_focus,
        right,
      })
    }
  }

  /// Moves the focus one position to the right.
  ///
  /// # Returns
  ///
  /// A new Zipper with the focus moved right, or `None` if already at the rightmost position
  pub fn move_right(&self) -> Option<Self> {
    if self.right.is_empty() {
      None
    } else {
      let mut left = self.left.clone();
      left.push(self.focus.clone());
      let mut right = self.right.clone();
      let new_focus = right.remove(0);

      Some(Zipper {
        left,
        focus: new_focus,
        right,
      })
    }
  }

  /// Replaces the focused element with a new value.
  ///
  /// # Arguments
  ///
  /// * `value` - The new value to place at the focus
  ///
  /// # Returns
  ///
  /// A new Zipper with the focused element replaced
  pub fn replace(&self, value: T) -> Self {
    Zipper {
      left: self.left.clone(),
      focus: value,
      right: self.right.clone(),
    }
  }

  /// Maps a function over the focused element.
  ///
  /// # Arguments
  ///
  /// * `f` - A function to apply to the focused element
  ///
  /// # Returns
  ///
  /// A new Zipper with the function applied to the focused element
  pub fn map_focus<F>(&self, f: F) -> Self
  where
    F: FnOnce(&T) -> T,
  {
    Zipper {
      left: self.left.clone(),
      focus: f(&self.focus),
      right: self.right.clone(),
    }
  }

  /// Transforms all elements in the Zipper using the provided function.
  ///
  /// # Arguments
  ///
  /// * `f` - A function to apply to each element
  ///
  /// # Returns
  ///
  /// A new Zipper with the function applied to all elements
  pub fn map<F, U>(&self, mut f: F) -> Zipper<U>
  where
    F: FnMut(&T) -> U,
    U: CloneableThreadSafe,
  {
    Zipper {
      left: self.left.iter().map(&mut f).collect(),
      focus: f(&self.focus),
      right: self.right.iter().map(&mut f).collect(),
    }
  }

  /// Returns the focus to the leftmost position.
  ///
  /// # Returns
  ///
  /// A new Zipper with the focus at the leftmost element
  pub fn start(&self) -> Self {
    let mut result = self.clone();
    while let Some(moved) = result.move_left() {
      result = moved;
    }
    result
  }

  /// Returns the focus to the rightmost position.
  ///
  /// # Returns
  ///
  /// A new Zipper with the focus at the rightmost element
  pub fn end(&self) -> Self {
    let mut result = self.clone();
    while let Some(moved) = result.move_right() {
      result = moved;
    }
    result
  }

  /// Converts the Zipper into a vector, preserving the order of elements.
  ///
  /// # Returns
  ///
  /// A vector containing all elements in the Zipper in left-to-right order
  pub fn into_vec(self) -> Vec<T> {
    let mut result = self.left.into_iter().rev().collect::<Vec<_>>();
    result.push(self.focus);
    result.extend(self.right);
    result
  }

  /// Returns the total number of elements in the Zipper.
  ///
  /// # Returns
  ///
  /// The total count of elements (left context + focus + right context)
  pub fn len(&self) -> usize {
    self.left.len() + 1 + self.right.len()
  }

  /// Returns the position of the focus within the Zipper.
  ///
  /// # Returns
  ///
  /// The zero-based index of the focused element
  pub fn position(&self) -> usize {
    self.left.len()
  }

  /// Checks if there are any elements to the left of the focus.
  ///
  /// # Returns
  ///
  /// `true` if there are elements to the left, `false` otherwise
  pub fn has_left(&self) -> bool {
    !self.left.is_empty()
  }

  /// Checks if there are any elements to the right of the focus.
  ///
  /// # Returns
  ///
  /// `true` if there are elements to the right, `false` otherwise
  pub fn has_right(&self) -> bool {
    !self.right.is_empty()
  }

  /// Returns an iterator over all elements in the Zipper.
  ///
  /// # Returns
  ///
  /// An iterator yielding references to each element in left-to-right order
  pub fn iter(&self) -> impl Iterator<Item = &T> {
    self
      .left
      .iter()
      .rev()
      .chain(std::iter::once(&self.focus))
      .chain(self.right.iter())
  }
}

impl<T: CloneableThreadSafe + Debug> Debug for Zipper<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    write!(
      f,
      "Zipper {{ left: {:?}, focus: {:?}, right: {:?} }}",
      self.left, self.focus, self.right
    )
  }
}

impl<T: CloneableThreadSafe + Display> Display for Zipper<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    write!(f, "[")?;
    for (i, item) in self.left.iter().rev().enumerate() {
      if i > 0 {
        write!(f, ", ")?;
      }
      write!(f, "{}", item)?;
    }

    if !self.left.is_empty() {
      write!(f, ", ")?;
    }

    write!(f, "[{}]", self.focus)?;

    if !self.right.is_empty() {
      write!(f, ", ")?;
    }

    for (i, item) in self.right.iter().enumerate() {
      if i > 0 {
        write!(f, ", ")?;
      }
      write!(f, "{}", item)?;
    }

    write!(f, "]")
  }
}
