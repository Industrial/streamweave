//! # Port Type System
//!
//! This module provides type-level port lists using tuples with trait-based
//! extraction for compile-time type safety. This enables nodes to have multiple
//! input and output ports while maintaining full type checking at compile time.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::graph::port::{GetPort, PortList};
//!
//! // Define a port list with two ports
//! type MyPorts = (i32, String);
//!
//! // Extract the first port type
//! type FirstPort = <MyPorts as GetPort<0>>::Type; // i32
//!
//! // Extract the second port type
//! type SecondPort = <MyPorts as GetPort<1>>::Type; // String
//!
//! // Empty ports
//! type NoPorts = ();
//! ```

/// Trait for extracting a port type from a port list by index.
///
/// This trait enables compile-time type extraction from tuples representing
/// port lists. The index `N` must be a compile-time constant.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::port::GetPort;
///
/// type Ports = (i32, String, bool);
///
/// // Extract types at compile time
/// type First = <Ports as GetPort<0>>::Type;  // i32
/// type Second = <Ports as GetPort<1>>::Type; // String
/// type Third = <Ports as GetPort<2>>::Type;   // bool
/// ```
pub trait GetPort<const N: usize> {
  /// The type of the port at index `N`.
  type Type;
}

/// Trait for working with port lists.
///
/// This trait provides metadata about port lists, including the number of ports.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::port::PortList;
///
/// type MyPorts = (i32, String);
/// assert_eq!(<MyPorts as PortList>::LEN, 2);
/// ```
pub trait PortList {
  /// The number of ports in this port list.
  const LEN: usize;
}

// Implement GetPort for empty tuple (0 ports)
impl GetPort<0> for () {
  type Type = ();
}

// Implement PortList for empty tuple
impl PortList for () {
  const LEN: usize = 0;
}

// Implement GetPort for single-element tuple (1 port)
impl<T0> GetPort<0> for (T0,) {
  type Type = T0;
}

// Implement PortList for single-element tuple
impl<T0> PortList for (T0,) {
  const LEN: usize = 1;
}

// Manual implementations for tuples up to 12 ports
// This ensures compile-time type safety and clear error messages

// Two ports: (T0, T1)
impl<T0, T1> GetPort<0> for (T0, T1) {
  type Type = T0;
}

impl<T0, T1> GetPort<1> for (T0, T1) {
  type Type = T1;
}

impl<T0, T1> PortList for (T0, T1) {
  const LEN: usize = 2;
}

// Three ports: (T0, T1, T2)
impl<T0, T1, T2> GetPort<0> for (T0, T1, T2) {
  type Type = T0;
}

impl<T0, T1, T2> GetPort<1> for (T0, T1, T2) {
  type Type = T1;
}

impl<T0, T1, T2> GetPort<2> for (T0, T1, T2) {
  type Type = T2;
}

impl<T0, T1, T2> PortList for (T0, T1, T2) {
  const LEN: usize = 3;
}

// Four ports: (T0, T1, T2, T3)
impl<T0, T1, T2, T3> GetPort<0> for (T0, T1, T2, T3) {
  type Type = T0;
}

impl<T0, T1, T2, T3> GetPort<1> for (T0, T1, T2, T3) {
  type Type = T1;
}

impl<T0, T1, T2, T3> GetPort<2> for (T0, T1, T2, T3) {
  type Type = T2;
}

impl<T0, T1, T2, T3> GetPort<3> for (T0, T1, T2, T3) {
  type Type = T3;
}

impl<T0, T1, T2, T3> PortList for (T0, T1, T2, T3) {
  const LEN: usize = 4;
}

// Five ports: (T0, T1, T2, T3, T4)
impl<T0, T1, T2, T3, T4> GetPort<0> for (T0, T1, T2, T3, T4) {
  type Type = T0;
}

impl<T0, T1, T2, T3, T4> GetPort<1> for (T0, T1, T2, T3, T4) {
  type Type = T1;
}

impl<T0, T1, T2, T3, T4> GetPort<2> for (T0, T1, T2, T3, T4) {
  type Type = T2;
}

impl<T0, T1, T2, T3, T4> GetPort<3> for (T0, T1, T2, T3, T4) {
  type Type = T3;
}

impl<T0, T1, T2, T3, T4> GetPort<4> for (T0, T1, T2, T3, T4) {
  type Type = T4;
}

impl<T0, T1, T2, T3, T4> PortList for (T0, T1, T2, T3, T4) {
  const LEN: usize = 5;
}

// Six ports: (T0, T1, T2, T3, T4, T5)
impl<T0, T1, T2, T3, T4, T5> GetPort<0> for (T0, T1, T2, T3, T4, T5) {
  type Type = T0;
}

impl<T0, T1, T2, T3, T4, T5> GetPort<1> for (T0, T1, T2, T3, T4, T5) {
  type Type = T1;
}

impl<T0, T1, T2, T3, T4, T5> GetPort<2> for (T0, T1, T2, T3, T4, T5) {
  type Type = T2;
}

impl<T0, T1, T2, T3, T4, T5> GetPort<3> for (T0, T1, T2, T3, T4, T5) {
  type Type = T3;
}

impl<T0, T1, T2, T3, T4, T5> GetPort<4> for (T0, T1, T2, T3, T4, T5) {
  type Type = T4;
}

impl<T0, T1, T2, T3, T4, T5> GetPort<5> for (T0, T1, T2, T3, T4, T5) {
  type Type = T5;
}

impl<T0, T1, T2, T3, T4, T5> PortList for (T0, T1, T2, T3, T4, T5) {
  const LEN: usize = 6;
}

// Seven ports: (T0, T1, T2, T3, T4, T5, T6)
impl<T0, T1, T2, T3, T4, T5, T6> GetPort<0> for (T0, T1, T2, T3, T4, T5, T6) {
  type Type = T0;
}

impl<T0, T1, T2, T3, T4, T5, T6> GetPort<1> for (T0, T1, T2, T3, T4, T5, T6) {
  type Type = T1;
}

impl<T0, T1, T2, T3, T4, T5, T6> GetPort<2> for (T0, T1, T2, T3, T4, T5, T6) {
  type Type = T2;
}

impl<T0, T1, T2, T3, T4, T5, T6> GetPort<3> for (T0, T1, T2, T3, T4, T5, T6) {
  type Type = T3;
}

impl<T0, T1, T2, T3, T4, T5, T6> GetPort<4> for (T0, T1, T2, T3, T4, T5, T6) {
  type Type = T4;
}

impl<T0, T1, T2, T3, T4, T5, T6> GetPort<5> for (T0, T1, T2, T3, T4, T5, T6) {
  type Type = T5;
}

impl<T0, T1, T2, T3, T4, T5, T6> GetPort<6> for (T0, T1, T2, T3, T4, T5, T6) {
  type Type = T6;
}

impl<T0, T1, T2, T3, T4, T5, T6> PortList for (T0, T1, T2, T3, T4, T5, T6) {
  const LEN: usize = 7;
}

// Eight ports: (T0, T1, T2, T3, T4, T5, T6, T7)
impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<0> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T0;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<1> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T1;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<2> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T2;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<3> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T3;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<4> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T4;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<5> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T5;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<6> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T6;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> GetPort<7> for (T0, T1, T2, T3, T4, T5, T6, T7) {
  type Type = T7;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> PortList for (T0, T1, T2, T3, T4, T5, T6, T7) {
  const LEN: usize = 8;
}

// Nine ports: (T0, T1, T2, T3, T4, T5, T6, T7, T8)
impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<0> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T0;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<1> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T1;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<2> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T2;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<3> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T3;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<4> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T4;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<5> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T5;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<6> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T6;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<7> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T7;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> GetPort<8> for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  type Type = T8;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> PortList for (T0, T1, T2, T3, T4, T5, T6, T7, T8) {
  const LEN: usize = 9;
}

// Ten ports: (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<0>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T0;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<1>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T1;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<2>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T2;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<3>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T3;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<4>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T4;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<5>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T5;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<6>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T6;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<7>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T7;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<8>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T8;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> GetPort<9>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)
{
  type Type = T9;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> PortList for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9) {
  const LEN: usize = 10;
}

// Eleven ports: (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<0>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T0;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<1>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T1;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<2>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T2;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<3>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T3;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<4>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T4;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<5>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T5;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<6>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T6;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<7>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T7;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<8>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T8;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<9>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T9;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> GetPort<10>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  type Type = T10;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> PortList
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
{
  const LEN: usize = 11;
}

// Twelve ports: (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<0>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T0;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<1>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T1;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<2>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T2;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<3>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T3;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<4>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T4;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<5>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T5;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<6>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T6;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<7>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T7;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<8>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T8;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<9>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T9;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<10>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T10;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> GetPort<11>
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  type Type = T11;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> PortList
  for (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
  const LEN: usize = 12;
}

/// Type alias for a single port.
///
/// This is a convenience alias for the common case of a node with a single port.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::port::SinglePort;
///
/// type MyPort = SinglePort<i32>; // Equivalent to (i32,)
/// ```
pub type SinglePort<T> = (T,);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_ports() {
    assert_eq!(<() as PortList>::LEN, 0);
    // Empty ports should return () when accessed
    type EmptyPort = <() as GetPort<0>>::Type;
    let _: EmptyPort = ();
  }

  #[test]
  fn test_single_port() {
    type Ports = (i32,);
    assert_eq!(<Ports as PortList>::LEN, 1);

    type First = <Ports as GetPort<0>>::Type;
    let _: First = 42i32;
  }

  #[test]
  fn test_two_ports() {
    type Ports = (i32, String);
    assert_eq!(<Ports as PortList>::LEN, 2);

    type First = <Ports as GetPort<0>>::Type;
    type Second = <Ports as GetPort<1>>::Type;

    let _: First = 42i32;
    let _: Second = "hello".to_string();
  }

  #[test]
  fn test_three_ports() {
    type Ports = (i32, String, bool);
    assert_eq!(<Ports as PortList>::LEN, 3);

    type First = <Ports as GetPort<0>>::Type;
    type Second = <Ports as GetPort<1>>::Type;
    type Third = <Ports as GetPort<2>>::Type;

    let _: First = 42i32;
    let _: Second = "hello".to_string();
    let _: Third = true;
  }

  #[test]
  fn test_four_ports() {
    type Ports = (i32, String, bool, f64);
    assert_eq!(<Ports as PortList>::LEN, 4);

    type First = <Ports as GetPort<0>>::Type;
    type Second = <Ports as GetPort<1>>::Type;
    type Third = <Ports as GetPort<2>>::Type;
    type Fourth = <Ports as GetPort<3>>::Type;

    let _: First = 42i32;
    let _: Second = "hello".to_string();
    let _: Third = true;
    let _: Fourth = std::f64::consts::PI;
  }

  #[test]
  fn test_single_port_alias() {
    type MyPort = SinglePort<i32>;
    assert_eq!(<MyPort as PortList>::LEN, 1);

    type PortType = <MyPort as GetPort<0>>::Type;
    let _: PortType = 42i32;
  }

  #[test]
  fn test_port_extraction_compile_time() {
    // This test verifies that port extraction works at compile time
    type Ports = (i32, String, bool);

    // These should all compile successfully
    fn extract_ports() {
      type P0 = <Ports as GetPort<0>>::Type;
      type P1 = <Ports as GetPort<1>>::Type;
      type P2 = <Ports as GetPort<2>>::Type;

      let _: P0 = 0i32;
      let _: P1 = String::new();
      let _: P2 = false;
    }

    extract_ports();
  }

  #[test]
  fn test_many_ports() {
    // Test that we can handle up to 12 ports
    type Ports = (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32);
    assert_eq!(<Ports as PortList>::LEN, 12);

    // Test extraction from various positions
    type First = <Ports as GetPort<0>>::Type;
    type Middle = <Ports as GetPort<6>>::Type;
    type Last = <Ports as GetPort<11>>::Type;

    let _: First = 0i32;
    let _: Middle = 6i32;
    let _: Last = 11i32;
  }
}
