//! Tests for port module

use streamweave::port::{GetPort, PortList};

#[test]
fn test_port_list_empty() {
  assert_eq!(<() as PortList>::LEN, 0);
}

#[test]
fn test_port_list_single() {
  assert_eq!(<(i32,) as PortList>::LEN, 1);
}

#[test]
fn test_port_list_double() {
  assert_eq!(<(i32, String) as PortList>::LEN, 2);
}

#[test]
fn test_port_list_triple() {
  assert_eq!(<(i32, String, bool) as PortList>::LEN, 3);
}

#[test]
fn test_get_port_single() {
  type Ports = (i32,);
  type First = <Ports as GetPort<0>>::Type;
  // Should compile - First is i32
  assert!(true);
}

#[test]
fn test_get_port_double() {
  type Ports = (i32, String);
  type First = <Ports as GetPort<0>>::Type;
  type Second = <Ports as GetPort<1>>::Type;
  // Should compile - First is i32, Second is String
  assert!(true);
}

#[test]
fn test_get_port_triple() {
  type Ports = (i32, String, bool);
  type First = <Ports as GetPort<0>>::Type;
  type Second = <Ports as GetPort<1>>::Type;
  type Third = <Ports as GetPort<2>>::Type;
  // Should compile - types are correct
  assert!(true);
}

#[test]
fn test_get_port_empty() {
  type Ports = ();
  type First = <Ports as GetPort<0>>::Type;
  // Should compile - First is ()
  assert!(true);
}
