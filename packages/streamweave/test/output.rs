//! Tests for output module

use futures::Stream;
use streamweave::output::Output;
use streamweave::producers::VecProducer;

#[test]
fn test_output_trait_implementation() {
  // Test that VecProducer implements Output
  // This is a compile-time check
  fn assert_output<T: Output>() {}
  assert_output::<VecProducer<i32>>();
  assert!(true);
}

#[test]
fn test_output_associated_types() {
  // Verify Output trait has correct associated types
  type VecProducerOutput = <VecProducer<i32> as Output>::Output;
  type VecProducerOutputStream = <VecProducer<i32> as Output>::OutputStream;

  // Should compile - this verifies the types exist
  assert!(true);
}
