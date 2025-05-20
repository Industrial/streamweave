//! Provides the [`Pair`] type for bidirectional transformations.
//!
//! The [`Pair`] type represents a pair of functions that map between two types,
//! enabling bidirectional transformations. This is useful for isomorphisms, lenses,
//! and other bidirectional patterns in functional programming.

use crate::types::threadsafe::CloneableThreadSafe;
use std::marker::PhantomData;
use std::sync::Arc;

/// A pair of functions that enable bidirectional transformations between two types.
///
/// The `Pair<A, B>` type contains a function `f: A -> B` and a function `g: B -> A`,
/// allowing for transformations in both directions.
///
/// # Examples
///
/// ```
/// use effect_core::types::pair::Pair;
///
/// // Create a pair that converts between inches and centimeters
/// let inches_to_cm = Pair::new(
///     |inches: f64| inches * 2.54,  // inches to cm
///     |cm: f64| cm / 2.54           // cm to inches
/// );
///
/// let inches = 10.0;
/// let cm = inches_to_cm.apply(inches);
/// assert_eq!(cm, 25.4);
///
/// let back_to_inches = inches_to_cm.unapply(cm);
/// assert_eq!(back_to_inches, inches);
/// ```
#[derive(Clone)]
pub struct Pair<A: CloneableThreadSafe, B: CloneableThreadSafe> {
    f: Arc<dyn Fn(A) -> B + Send + Sync + 'static>,
    g: Arc<dyn Fn(B) -> A + Send + Sync + 'static>,
    _phantom: PhantomData<(A, B)>,
}

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Pair<A, B> {
    /// Creates a new `Pair` with the given forward and backward functions.
    ///
    /// # Arguments
    ///
    /// * `f` - The forward function that maps from `A` to `B`.
    /// * `g` - The backward function that maps from `B` to `A`.
    ///
    /// # Returns
    ///
    /// A new `Pair<A, B>` containing the provided functions.
    pub fn new<F, G>(f: F, g: G) -> Self
    where
        F: Fn(A) -> B + Send + Sync + 'static,
        G: Fn(B) -> A + Send + Sync + 'static,
    {
        Self {
            f: Arc::new(f),
            g: Arc::new(g),
            _phantom: PhantomData,
        }
    }

    /// Applies the forward function to transform from type `A` to type `B`.
    ///
    /// # Arguments
    ///
    /// * `x` - A value of type `A` to transform.
    ///
    /// # Returns
    ///
    /// A value of type `B` resulting from applying the forward function.
    pub fn apply(&self, x: A) -> B {
        (self.f)(x)
    }

    /// Applies the backward function to transform from type `B` to type `A`.
    ///
    /// # Arguments
    ///
    /// * `y` - A value of type `B` to transform.
    ///
    /// # Returns
    ///
    /// A value of type `A` resulting from applying the backward function.
    pub fn unapply(&self, y: B) -> A {
        (self.g)(y)
    }
} 