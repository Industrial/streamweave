use futures::Stream;

/// Trait for components that can provide input streams.
///
/// This trait defines the interface for components that produce data streams.
/// It is implemented by producers and transformers that output data.
pub trait Input {
  /// The type of items produced by this input stream.
  type Input;
  /// The input stream type that yields items of type `Self::Input`.
  type InputStream: Stream<Item = Self::Input> + Send;
}
