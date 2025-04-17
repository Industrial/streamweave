//! Bifunctor trait and implementations.
//!
//! A bifunctor is a functor that takes two type parameters and can map over both of them.
//! It extends the Category trait to provide operations on pairs of values.
//!
//! # Examples
//!
//! ```rust
//! use effect_core::bifunctor::Bifunctor;
//! use effect_core::category::Morphism;
//!
//! let f = |x: i32| x * 2;
//! let g = |x: i32| x + 1;
//! let bimap = <Morphism<(), ()> as Bifunctor>::bimap(f, g);
//! let result = bimap.apply((3, 4));
//! assert_eq!(result, (6, 5));
//! ```

use crate::category::Morphism;

use super::category::Category;

/// A bifunctor is a functor that takes two type parameters and can map over both of them.
/// It extends the Category trait to provide operations on pairs of values.
///
/// The Bifunctor trait provides three main operations:
/// - `bimap`: Maps over both components of a pair using two functions
/// - `first`: Maps over the first component of a pair
/// - `second`: Maps over the second component of a pair
///
/// # Laws
///
/// A bifunctor must satisfy the following laws:
///
/// 1. Identity:
///    ```rust
///    bimap(id, id) = id
///    ```
///
/// 2. Composition:
///    ```rust
///    bimap(f1, g1) . bimap(f2, g2) = bimap(f1 . f2, g1 . g2)
///    ```
///
/// 3. First/Second Commutativity:
///    ```rust
///    first(f) . second(g) = second(g) . first(f)
///    ```
pub trait Bifunctor<A: Send + Sync + 'static, B: Send + Sync + 'static>: Category<A, B> {
  /// Maps over both components of a pair using two functions.
  ///
  /// # Arguments
  ///
  /// * `f` - A function to map over the first component
  /// * `g` - A function to map over the second component
  ///
  /// # Returns
  ///
  /// A morphism that maps pairs of values using the provided functions.
  fn bimap<C, D, F, G>(f: F, g: G) -> Self::Morphism<(A, B), (C, D)>
  where
    C: Send + Sync + 'static,
    D: Send + Sync + 'static,
    F: Fn(A) -> C + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static;

  /// Maps over the first component of a pair.
  ///
  /// # Arguments
  ///
  /// * `f` - A function to map over the first component
  ///
  /// # Returns
  ///
  /// A morphism that maps the first component of pairs using the provided function.
  fn first<C, F>(f: F) -> Self::Morphism<(A, B), (C, B)>
  where
    C: Send + Sync + 'static,
    F: Fn(A) -> C + Send + Sync + 'static,
  {
    Self::bimap(f, |x| x)
  }

  /// Maps over the second component of a pair.
  ///
  /// # Arguments
  ///
  /// * `g` - A function to map over the second component
  ///
  /// # Returns
  ///
  /// A morphism that maps the second component of pairs using the provided function.
  fn second<D, G>(g: G) -> Self::Morphism<(A, B), (A, D)>
  where
    D: Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    Self::bimap(|x| x, g)
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Bifunctor<A, B> for Morphism<A, B> {
  fn bimap<C, D, F, G>(f: F, g: G) -> Self::Morphism<(A, B), (C, D)>
  where
    C: Send + Sync + 'static,
    D: Send + Sync + 'static,
    F: Fn(A) -> C + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    Morphism::new(move |(x, y)| (f(x), g(y)))
  }
}

#[cfg(test)]
mod tests {
  use super::super::category::Morphism;
  use super::*;
  use proptest::prelude::*;

  proptest! {
      #[test]
      fn test_bifunctor_bimap(a: i32, b: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let bimap = <Morphism<i32, i32> as Bifunctor<i32, i32>>::bimap(f, g);
          let expected = (
              a.checked_mul(2).unwrap_or(i32::MAX),
              b.checked_add(1).unwrap_or(i32::MAX)
          );
          assert_eq!(bimap.apply((a, b)), expected);
      }

      #[test]
      fn test_bifunctor_first(a: i32, b: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let first = <Morphism<i32, i32> as Bifunctor<i32, i32>>::first(f);
          let expected = (a.checked_mul(2).unwrap_or(i32::MAX), b);
          assert_eq!(first.apply((a, b)), expected);
      }

      #[test]
      fn test_bifunctor_second(a: i32, b: i32) {
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let second = <Morphism<i32, i32> as Bifunctor<i32, i32>>::second(g);
          let expected = (a, b.checked_add(1).unwrap_or(i32::MAX));
          assert_eq!(second.apply((a, b)), expected);
      }

      #[test]
      fn test_bifunctor_laws(a: i32, b: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let h = |x: i32| x.checked_mul(3).unwrap_or(i32::MAX);
          let k = |x: i32| x.checked_sub(1).unwrap_or(i32::MIN);

          // Test first . second = second . first
          let first_then_second = <Morphism<i32, i32> as Category<i32, i32>>::compose(
              <Morphism<i32, i32> as Bifunctor<i32, i32>>::first(f),
              <Morphism<i32, i32> as Bifunctor<i32, i32>>::second(g)
          );

          let second_then_first = <Morphism<i32, i32> as Category<i32, i32>>::compose(
              <Morphism<i32, i32> as Bifunctor<i32, i32>>::second(g),
              <Morphism<i32, i32> as Bifunctor<i32, i32>>::first(f)
          );

          let input = (a, b);
          let result1 = first_then_second.apply(input.clone());
          let result2 = second_then_first.apply(input);
          assert_eq!(result1, result2);

          // Test bimap composition
          let bimap1 = <Morphism<i32, i32> as Bifunctor<i32, i32>>::bimap(f, g);
          let bimap2 = <Morphism<i32, i32> as Bifunctor<i32, i32>>::bimap(h, k);
          let composed = <Morphism<i32, i32> as Category<i32, i32>>::compose(bimap1, bimap2);

          let expected = (
              a.checked_mul(2)
                  .and_then(|x| x.checked_mul(3))
                  .unwrap_or(i32::MAX),
              b.checked_add(1)
                  .and_then(|x| x.checked_sub(1))
                  .unwrap_or(i32::MIN)
          );
          assert_eq!(composed.apply((a, b)), expected);
      }
  }
}
