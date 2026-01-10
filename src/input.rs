use futures::Stream;

/// Trait for components that can provide input streams.
///
/// This trait defines the interface for components that produce data streams.
/// It is implemented by transformers and consumers that receive data.
///
/// **Note**: In StreamWeave, `Input::Input` is `Message<T>` where `T` is the payload type.
/// All items flowing through StreamWeave are wrapped in messages.
pub trait Input
where
  Self::Input: Send + 'static,
{
  /// The type of items produced by this input stream.
  /// This is `Message<T>` where `T` is the payload type.
  type Input;
  /// The input stream type that yields items of type `Self::Input`.
  /// This yields `Message<T>` items.
  type InputStream: Stream<Item = Self::Input> + Send + 'static;
}
