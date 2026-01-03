use futures::Stream;

/// Trait for components that can produce output streams.
///
/// This trait defines the interface for components that generate data streams.
/// It is implemented by producers and transformers that output data.
///
/// **Note**: In StreamWeave, `Output::Output` is `Message<T>` where `T` is the payload type.
/// All items flowing through StreamWeave are wrapped in messages.
pub trait Output {
  /// The type of items produced by this output stream.
  /// This is `Message<T>` where `T` is the payload type.
  type Output;
  /// The output stream type that yields items of type `Self::Output`.
  /// This yields `Message<T>` items.
  type OutputStream: Stream<Item = Self::Output> + Send;
}
