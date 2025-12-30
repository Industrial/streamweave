//! Connection tests
//!
//! This module provides integration tests for connection types and validation.

use streamweave_graph::{
  CompatibleWith, Connection, ConnectionError, ConsumerNode, HasInputPort, HasOutputPort,
  ProducerNode,
};
use streamweave_vec::{VecConsumer, VecProducer};

#[test]
fn test_producer_to_consumer_connection() {
  // Valid connection: Producer<i32> -> Consumer<i32>
  type Source = ProducerNode<VecProducer<i32>, (i32,)>;
  type Target = ConsumerNode<VecConsumer<i32>, (i32,)>;

  let _connection: Connection<Source, Target, 0, 0> = Connection::new();
}

#[test]
fn test_connection_port_accessors() {
  type Source = ProducerNode<VecProducer<i32>, (i32,)>;
  type Target = ConsumerNode<VecConsumer<i32>, (i32,)>;

  let _connection: Connection<Source, Target, 0, 0> = Connection::new();
  assert_eq!(Connection::<Source, Target, 0, 0>::source_port(), 0);
  assert_eq!(Connection::<Source, Target, 0, 0>::target_port(), 0);
}

#[test]
fn test_connection_error_display() {
  let error = ConnectionError::InvalidPortIndex {
    index: 5,
    max_index: 3,
  };
  assert_eq!(
    error.to_string(),
    "Invalid port index: 5 (max valid index: 3)"
  );

  let error = ConnectionError::TypeMismatch {
    message: "Expected i32, got String".to_string(),
  };
  assert_eq!(error.to_string(), "Type mismatch: Expected i32, got String");
}

#[test]
fn test_has_output_port_producer() {
  type Node = ProducerNode<VecProducer<i32>, (i32,)>;
  type OutputType = <Node as HasOutputPort<0>>::OutputType;
  let _: OutputType = 42i32;
}

#[test]
fn test_has_input_port_consumer() {
  type Node = ConsumerNode<VecConsumer<String>, (String,)>;
  type InputType = <Node as HasInputPort<0>>::InputType;
  let _: InputType = "hello".to_string();
}

#[test]
fn test_compatible_with() {
  // i32 is compatible with i32
  fn check_compatibility<T: CompatibleWith<i32>>(_: T) {}
  check_compatibility::<i32>(42);
}

#[test]
fn test_connection_default() {
  type Source = ProducerNode<VecProducer<i32>, (i32,)>;
  type Target = ConsumerNode<VecConsumer<i32>, (i32,)>;

  let connection: Connection<Source, Target, 0, 0> = Connection::default();
  assert_eq!(Connection::<Source, Target, 0, 0>::source_port(), 0);
  assert_eq!(Connection::<Source, Target, 0, 0>::target_port(), 0);
}

#[test]
fn test_connection_error_clone() {
  let error1 = ConnectionError::InvalidPortIndex {
    index: 5,
    max_index: 3,
  };
  let error2 = error1.clone();

  assert_eq!(error1, error2);
  assert_eq!(error1.to_string(), error2.to_string());
}

#[test]
fn test_connection_error_partial_eq() {
  let error1 = ConnectionError::InvalidPortIndex {
    index: 5,
    max_index: 3,
  };
  let error2 = ConnectionError::InvalidPortIndex {
    index: 5,
    max_index: 3,
  };
  let error3 = ConnectionError::InvalidPortIndex {
    index: 4,
    max_index: 3,
  };

  assert_eq!(error1, error2);
  assert_ne!(error1, error3);
}

#[test]
fn test_connection_error_debug() {
  let error = ConnectionError::TypeMismatch {
    message: "test".to_string(),
  };
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}
