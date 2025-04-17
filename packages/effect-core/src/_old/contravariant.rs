//! Contravariant trait and implementations.
//!
//! A contravariant functor is a functor that reverses the direction of morphisms.
//! It provides operations for mapping over the input type of a function.

use crate::category::Morphism;

use super::category::Category;

/// A contravariant functor is a functor that reverses the direction of morphisms.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameters A and B must implement Send + Sync + 'static
/// - The function F must implement Send + Sync + 'static
pub trait Contravariant<A: Send + Sync + 'static, B: Send + Sync + 'static>:
  Category<A, B>
{
  /// Maps over the input type of a function, reversing the direction of the morphism.
  ///
  /// # Arguments
  ///
  /// * `f` - A function to map over the input type
  ///
  /// # Returns
  ///
  /// A morphism that maps the input type using the provided function.
  fn contramap<C, F>(f: F) -> Self::Morphism<C, A>
  where
    C: Send + Sync + 'static,
    F: Fn(C) -> B + Send + Sync + 'static;
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Contravariant<A, B> for Morphism<A, B> {
  fn contramap<C, F>(f: F) -> Self::Morphism<C, A>
  where
    C: Send + Sync + 'static,
    F: Fn(C) -> B + Send + Sync + 'static,
  {
    Morphism::new(move |x| {
      let result = f(x);
      // We need to convert from B to A here
      // Since we don't have a direct conversion, we'll use a default value
      // This is a placeholder - you'll need to provide the actual conversion logic
      Default::default()
    })
  }
}

#[cfg(test)]
mod tests {
  use super::super::category::Morphism;
  use super::*;
  use proptest::prelude::*;

  proptest! {
      #[test]
      fn test_contravariant_contramap(a: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let contramap = <Morphism<i32, i32> as Contravariant<i32, i32>>::contramap::<i32, _>(f);
          let composed = <Morphism<i32, i32> as Category<i32, i32>>::compose(contramap, Morphism::new(g));
          let expected = a.checked_mul(2)
              .and_then(|x| x.checked_add(1))
              .unwrap_or(i32::MAX);
          assert_eq!(composed.apply(a), expected);
      }

      #[test]
      fn test_contravariant_laws(a: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let h = |x: i32| x.checked_mul(3).unwrap_or(i32::MAX);

          // Test identity law
          let id = <Morphism<i32, i32> as Category<i32, i32>>::id::<i32>();
          let contramap_id = <Morphism<i32, i32> as Contravariant<i32, i32>>::contramap::<i32, _>(|x| x);
          let composed_id = <Morphism<i32, i32> as Category<i32, i32>>::compose(contramap_id, id);
          assert_eq!(composed_id.apply(a), a);

          // Test composition law
          let contramap_f = <Morphism<i32, i32> as Contravariant<i32, i32>>::contramap::<i32, _>(f);
          let contramap_g = <Morphism<i32, i32> as Contravariant<i32, i32>>::contramap::<i32, _>(g);
          let contramap_h = <Morphism<i32, i32> as Contravariant<i32, i32>>::contramap::<i32, _>(h);

          let comp1 = <Morphism<i32, i32> as Category<i32, i32>>::compose(
              <Morphism<i32, i32> as Category<i32, i32>>::compose(contramap_f.clone(), contramap_g.clone()),
              contramap_h.clone()
          );

          let comp2 = <Morphism<i32, i32> as Category<i32, i32>>::compose(
              contramap_f,
              <Morphism<i32, i32> as Category<i32, i32>>::compose(contramap_g, contramap_h)
          );

          let expected = a.checked_mul(2)
              .and_then(|x| x.checked_add(1))
              .and_then(|x| x.checked_mul(3))
              .unwrap_or(i32::MAX);
          assert_eq!(comp1.apply(a), expected);
          assert_eq!(comp2.apply(a), expected);
      }
  }
}
