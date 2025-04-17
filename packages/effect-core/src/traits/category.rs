/// A category is a collection of objects and morphisms (arrows) between them.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The type parameter U must implement Send + Sync + 'static
/// - The composition function must implement Send + Sync + 'static
pub trait Category<T: Send + Sync + 'static, U: Send + Sync + 'static> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static>: Send + Sync + 'static;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A>;

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C>;

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static;

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)>;

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)>;
}
