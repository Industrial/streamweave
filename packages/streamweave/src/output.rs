use futures::Stream;

/// Trait for components that can produce output streams.
///
/// This trait defines the interface for components that generate data streams.
/// It is implemented by producers and transformers that output data.
pub trait Output {
  /// The type of items produced by this output stream.
  type Output;
  /// The output stream type that yields items of type `Self::Output`.
  type OutputStream: Stream<Item = Self::Output> + Send;
}
