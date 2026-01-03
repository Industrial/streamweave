//! Port type system tests
//!
//! This module provides tests for port type system functionality.
//! Since port.rs is a re-export module, these tests verify that
//! the re-exports work correctly and that port types are accessible.

use streamweave::graph::port::*;
use streamweave::port::*;

#[test]
fn test_port_types_accessible() {
  // Test that port types are accessible through the re-export
  // This is a compile-time test - if this compiles, the re-export works

  // Test PortList trait is accessible
  fn _test_port_list<T: PortList>() {}

  // Test GetPort trait is accessible
  fn _test_get_port<T: PortList, const N: usize>()
  where
    T: GetPort<N>,
  {
  }

  // Test that we can use port types
  type SinglePort = (i32,);
  let _: <SinglePort as GetPort<0>>::Type = 42i32;
}

#[test]
fn test_port_list_length() {
  // Test that PortList::LEN works correctly
  assert_eq!(<(i32,) as PortList>::LEN, 1);
  assert_eq!(<(i32, String) as PortList>::LEN, 2);
  assert_eq!(<(i32, String, bool) as PortList>::LEN, 3);
}

#[test]
fn test_get_port_type() {
  // Test that GetPort trait works correctly
  type TwoPorts = (i32, String);

  // Port 0 should be i32
  let _: <TwoPorts as GetPort<0>>::Type = 42i32;

  // Port 1 should be String
  let _: <TwoPorts as GetPort<1>>::Type = "hello".to_string();
}

#[test]
fn test_port_types_work_with_nodes() {
  use streamweave::graph::{ConsumerNode, ProducerNode};
  use streamweave_vec::{VecConsumer, VecProducer};

  // Test that port types work with node creation
  let producer = VecProducer::new(vec![1, 2, 3]);
  let _node: ProducerNode<_, (i32,)> = ProducerNode::new("test".to_string(), producer);

  let consumer = VecConsumer::<i32>::new();
  let _node: ConsumerNode<_, (i32,)> = ConsumerNode::new("test".to_string(), consumer);
}
