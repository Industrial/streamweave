//! # Port System Test Suite
//!
//! Comprehensive test suite for the port system, including port list length
//! calculations, type extraction via `GetPort`, and single port type aliases.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **PortList Length**: Tests for empty, single, and multi-element port lists (up to 12 ports)
//! - **GetPort Type Extraction**: Type-level extraction of specific port types from port lists
//! - **SinglePort Type Alias**: Verification that `SinglePort<T>` is equivalent to `(T,)`
//! - **Type Compatibility**: Tests with different payload types and repeated types
//!
//! ## Test Organization
//!
//! Tests are organized into three main sections:
//!
//! 1. **PortList LEN Tests**: Verify the `LEN` associated constant for various port list sizes
//! 2. **GetPort Type Extraction Tests**: Verify type extraction at different indices
//! 3. **SinglePort Type Alias Tests**: Verify the convenience type alias works correctly
//!
//! ## Key Concepts
//!
//! - **PortList**: A trait for tuples of `Message<T>` types representing multiple ports
//! - **GetPort**: A trait for extracting a specific port type by index from a port list
//! - **SinglePort**: A type alias for single-port scenarios: `type SinglePort<T> = (T,);`
//!
//! ## Usage
//!
//! These tests ensure that the port system's type-level operations work correctly,
//! enabling compile-time validation of graph connections and port compatibility.

use crate::message::Message;
use crate::port::{GetPort, PortList, SinglePort};

// ============================================================================
// PortList LEN Tests
// ============================================================================

#[test]
fn test_port_list_empty() {
  assert_eq!(<() as PortList>::LEN, 0);
}

#[test]
fn test_port_list_single() {
  assert_eq!(<(Message<i32>,) as PortList>::LEN, 1);
}

#[test]
fn test_port_list_two() {
  assert_eq!(<(Message<i32>, Message<String>) as PortList>::LEN, 2);
}

#[test]
fn test_port_list_three() {
  assert_eq!(
    <(Message<i32>, Message<String>, Message<bool>) as PortList>::LEN,
    3
  );
}

#[test]
fn test_port_list_four() {
  assert_eq!(
    <(Message<i32>, Message<String>, Message<bool>, Message<f64>,) as PortList>::LEN,
    4
  );
}

#[test]
fn test_port_list_five() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
    ) as PortList>::LEN,
    5
  );
}

#[test]
fn test_port_list_six() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
      Message<i64>,
    ) as PortList>::LEN,
    6
  );
}

#[test]
fn test_port_list_seven() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
      Message<i64>,
      Message<u32>,
    ) as PortList>::LEN,
    7
  );
}

#[test]
fn test_port_list_eight() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
      Message<i64>,
      Message<u32>,
      Message<i32>,
    ) as PortList>::LEN,
    8
  );
}

#[test]
fn test_port_list_nine() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
      Message<i64>,
      Message<u32>,
      Message<i32>,
      Message<String>,
    ) as PortList>::LEN,
    9
  );
}

#[test]
fn test_port_list_ten() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
      Message<i64>,
      Message<u32>,
      Message<i32>,
      Message<String>,
      Message<bool>,
    ) as PortList>::LEN,
    10
  );
}

#[test]
fn test_port_list_eleven() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
      Message<i64>,
      Message<u32>,
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f32>,
    ) as PortList>::LEN,
    11
  );
}

#[test]
fn test_port_list_twelve() {
  assert_eq!(
    <(
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f64>,
      Message<u64>,
      Message<i64>,
      Message<u32>,
      Message<i32>,
      Message<String>,
      Message<bool>,
      Message<f32>,
      Message<u16>,
    ) as PortList>::LEN,
    12
  );
}

// ============================================================================
// GetPort Type Extraction Tests
// ============================================================================

#[test]
fn test_get_port_empty_zero() {
  type PortType = <() as GetPort<0>>::Type;
  // Just verify it compiles - should be ()
  let _port: PortType = ();
}

#[test]
fn test_get_port_single_zero() {
  type PortType = <(Message<i32>,) as GetPort<0>>::Type;
  // Should be Message<i32>
  let _port: PortType = Message::new(42, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_two_zero() {
  type PortType = <(Message<i32>, Message<String>) as GetPort<0>>::Type;
  // Should be Message<i32>
  let _port: PortType = Message::new(42, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_two_one() {
  type PortType = <(Message<i32>, Message<String>) as GetPort<1>>::Type;
  // Should be Message<String>
  let _port: PortType = Message::new("test".to_string(), crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_three_zero() {
  type PortType = <(Message<i32>, Message<String>, Message<bool>) as GetPort<0>>::Type;
  // Should be Message<i32>
  let _port: PortType = Message::new(42, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_three_one() {
  type PortType = <(Message<i32>, Message<String>, Message<bool>) as GetPort<1>>::Type;
  // Should be Message<String>
  let _port: PortType = Message::new("test".to_string(), crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_three_two() {
  type PortType = <(Message<i32>, Message<String>, Message<bool>) as GetPort<2>>::Type;
  // Should be Message<bool>
  let _port: PortType = Message::new(true, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_four_all_indices() {
  type Port0 = <(Message<i32>, Message<String>, Message<bool>, Message<f64>) as GetPort<0>>::Type;
  type Port1 = <(Message<i32>, Message<String>, Message<bool>, Message<f64>) as GetPort<1>>::Type;
  type Port2 = <(Message<i32>, Message<String>, Message<bool>, Message<f64>) as GetPort<2>>::Type;
  type Port3 = <(Message<i32>, Message<String>, Message<bool>, Message<f64>) as GetPort<3>>::Type;

  let _port0: Port0 = Message::new(42, crate::message::MessageId::new_uuid());
  let _port1: Port1 = Message::new("test".to_string(), crate::message::MessageId::new_uuid());
  let _port2: Port2 = Message::new(true, crate::message::MessageId::new_uuid());
  let _port3: Port3 = Message::new(std::f64::consts::PI, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_five_middle() {
  type Port2 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
  ) as GetPort<2>>::Type;
  // Should be Message<bool>
  let _port: Port2 = Message::new(true, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_six_last() {
  type Port5 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
  ) as GetPort<5>>::Type;
  // Should be Message<i64>
  let _port: Port5 = Message::new(123i64, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_seven_first() {
  type Port0 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
  ) as GetPort<0>>::Type;
  // Should be Message<i32>
  let _port: Port0 = Message::new(42, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_eight_last() {
  type Port7 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
  ) as GetPort<7>>::Type;
  // Should be Message<i32>
  let _port: Port7 = Message::new(42, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_nine_all_indices() {
  type Port0 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
  ) as GetPort<0>>::Type;
  type Port8 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
  ) as GetPort<8>>::Type;

  let _port0: Port0 = Message::new(42, crate::message::MessageId::new_uuid());
  let _port8: Port8 = Message::new("test".to_string(), crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_ten_all_indices() {
  type Port0 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
    Message<bool>,
  ) as GetPort<0>>::Type;
  type Port9 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
    Message<bool>,
  ) as GetPort<9>>::Type;

  let _port0: Port0 = Message::new(42, crate::message::MessageId::new_uuid());
  let _port9: Port9 = Message::new(true, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_eleven_all_indices() {
  type Port0 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f32>,
  ) as GetPort<0>>::Type;
  type Port10 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f32>,
  ) as GetPort<10>>::Type;

  let _port0: Port0 = Message::new(42, crate::message::MessageId::new_uuid());
  let _port10: Port10 = Message::new(std::f32::consts::PI, crate::message::MessageId::new_uuid());
}

#[test]
fn test_get_port_twelve_all_indices() {
  type Port0 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f32>,
    Message<u16>,
  ) as GetPort<0>>::Type;
  type Port11 = <(
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f64>,
    Message<u64>,
    Message<i64>,
    Message<u32>,
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<f32>,
    Message<u16>,
  ) as GetPort<11>>::Type;

  let _port0: Port0 = Message::new(42, crate::message::MessageId::new_uuid());
  let _port11: Port11 = Message::new(123u16, crate::message::MessageId::new_uuid());
}

// ============================================================================
// SinglePort Type Alias Tests
// ============================================================================

#[test]
fn test_single_port_type_alias() {
  type MyPort = SinglePort<Message<i32>>;
  // Should be equivalent to (Message<i32>,)
  assert_eq!(<MyPort as PortList>::LEN, 1);

  type PortType = <MyPort as GetPort<0>>::Type;
  let _port: PortType = Message::new(42, crate::message::MessageId::new_uuid());
}

#[test]
fn test_single_port_string() {
  type MyPort = SinglePort<Message<String>>;
  assert_eq!(<MyPort as PortList>::LEN, 1);

  type PortType = <MyPort as GetPort<0>>::Type;
  let _port: PortType = Message::new("test".to_string(), crate::message::MessageId::new_uuid());
}

// ============================================================================
// Type Compatibility Tests
// ============================================================================

#[test]
fn test_port_types_with_different_payload_types() {
  type Ports = (
    Message<i32>,
    Message<String>,
    Message<bool>,
    Message<Vec<u8>>,
  );

  assert_eq!(<Ports as PortList>::LEN, 4);

  type Port0 = <Ports as GetPort<0>>::Type;
  type Port1 = <Ports as GetPort<1>>::Type;
  type Port2 = <Ports as GetPort<2>>::Type;
  type Port3 = <Ports as GetPort<3>>::Type;

  let _port0: Port0 = Message::new(42, crate::message::MessageId::new_uuid());
  let _port1: Port1 = Message::new("test".to_string(), crate::message::MessageId::new_uuid());
  let _port2: Port2 = Message::new(true, crate::message::MessageId::new_uuid());
  let _port3: Port3 = Message::new(vec![1, 2, 3], crate::message::MessageId::new_uuid());
}

#[test]
fn test_port_list_with_same_type_multiple_times() {
  type Ports = (Message<i32>, Message<i32>, Message<i32>);

  assert_eq!(<Ports as PortList>::LEN, 3);

  type Port0 = <Ports as GetPort<0>>::Type;
  type Port1 = <Ports as GetPort<1>>::Type;
  type Port2 = <Ports as GetPort<2>>::Type;

  // All should be Message<i32>
  let _port0: Port0 = Message::new(1, crate::message::MessageId::new_uuid());
  let _port1: Port1 = Message::new(2, crate::message::MessageId::new_uuid());
  let _port2: Port2 = Message::new(3, crate::message::MessageId::new_uuid());
}
