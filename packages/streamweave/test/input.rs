//! Tests for input module

use futures::Stream;
use streamweave::consumers::VecConsumer;
use streamweave::input::Input;

#[test]
fn test_input_trait_implementation() {
  // Test that VecConsumer implements Input
  // This is a compile-time check
  fn assert_input<T: Input>() {}
  assert_input::<VecConsumer<i32>>();
  assert!(true);
}

#[test]
fn test_input_associated_types() {
  // Verify Input trait has correct associated types
  type VecConsumerInput = <VecConsumer<i32> as Input>::Input;
  type VecConsumerInputStream = <VecConsumer<i32> as Input>::InputStream;

  // Should compile - this verifies the types exist
  assert!(true);
}
