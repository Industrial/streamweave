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

#[cfg(test)]
mod tests {
  use super::*;
  use futures::{StreamExt, stream};
  use std::pin::Pin;

  // Test implementation for numeric output
  struct NumberOutput {
    data: Vec<i32>,
  }

  impl Output for NumberOutput {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
  }

  // Test implementation for string output
  struct TextOutput {
    data: String,
  }

  impl Output for TextOutput {
    type Output = String;
    type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
  }

  #[tokio::test]
  async fn test_number_output_stream() {
    let output = NumberOutput {
      data: vec![1, 2, 3],
    };

    // Verify type constraints are met
    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(output.data));

    // Test stream functionality
    let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
    assert_eq!(sum, 6);
  }

  #[tokio::test]
  async fn test_text_output_stream() {
    let output = TextOutput {
      data: "hello".to_string(),
    };

    // Verify type constraints are met
    let stream: Pin<Box<dyn Stream<Item = String> + Send>> =
      Box::pin(stream::iter(output.data.chars().map(|c| c.to_string())));

    // Test stream functionality
    let result = stream.collect::<String>().await;
    assert_eq!(result, "hello");
  }

  #[tokio::test]
  async fn test_output_stream_send() {
    let output = NumberOutput {
      data: vec![1, 2, 3],
    };

    let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(output.data));

    // Verify we can send the stream between threads
    let handle = tokio::spawn(async move {
      let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
      assert_eq!(sum, 6);
    });

    handle.await.unwrap();
  }

  #[test]
  fn test_output_trait_bounds() {
    fn takes_output<T: Output>(_: T) {}

    takes_output(NumberOutput { data: vec![] });
    takes_output(TextOutput {
      data: String::new(),
    });
  }

  #[test]
  fn test_output_static_bounds() {
    fn static_output_fn() -> impl Output {
      NumberOutput { data: vec![] }
    }

    let _output = static_output_fn();
  }

  #[tokio::test]
  async fn test_output_stream_composition() {
    let output1 = NumberOutput {
      data: vec![1, 2, 3],
    };
    let output2 = NumberOutput {
      data: vec![4, 5, 6],
    };

    // Test that we can compose output streams
    let stream1: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(output1.data));
    let stream2: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(output2.data));

    let combined = Box::pin(stream::select(stream1, stream2));
    let sum = combined.fold(0, |acc, x| async move { acc + x }).await;
    assert_eq!(sum, 21); // 1 + 2 + 3 + 4 + 5 + 6
  }
}
