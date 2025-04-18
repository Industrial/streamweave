use crate::types::threadsafe::CloneableThreadSafe;

/// A category is a collection of objects and morphisms (arrows) between them.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The type parameter U must implement Send + Sync + 'static
/// - The composition function must implement Send + Sync + 'static
pub trait Category<T: CloneableThreadSafe, U: CloneableThreadSafe> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe>: CloneableThreadSafe;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A>;

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C>;

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe;

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)>;

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)>;
}
