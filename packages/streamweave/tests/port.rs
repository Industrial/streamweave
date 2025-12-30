//! Tests for Port system

use streamweave::port::{GetPort, PortList, SinglePort};

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

#[test]
fn test_five_ports() {
  type Ports = (i32, String, bool, f64, u8);
  assert_eq!(<Ports as PortList>::LEN, 5);

  type First = <Ports as GetPort<0>>::Type;
  type Second = <Ports as GetPort<1>>::Type;
  type Third = <Ports as GetPort<2>>::Type;
  type Fourth = <Ports as GetPort<3>>::Type;
  type Fifth = <Ports as GetPort<4>>::Type;

  let _: First = 42i32;
  let _: Second = "hello".to_string();
  let _: Third = true;
  let _: Fourth = std::f64::consts::PI;
  let _: Fifth = 255u8;
}

#[test]
fn test_six_ports() {
  type Ports = (i32, i32, i32, i32, i32, i32);
  assert_eq!(<Ports as PortList>::LEN, 6);

  type First = <Ports as GetPort<0>>::Type;
  type Last = <Ports as GetPort<5>>::Type;

  let _: First = 0i32;
  let _: Last = 5i32;
}

#[test]
fn test_seven_ports() {
  type Ports = (i32, i32, i32, i32, i32, i32, i32);
  assert_eq!(<Ports as PortList>::LEN, 7);

  type First = <Ports as GetPort<0>>::Type;
  type Middle = <Ports as GetPort<3>>::Type;
  type Last = <Ports as GetPort<6>>::Type;

  let _: First = 0i32;
  let _: Middle = 3i32;
  let _: Last = 6i32;
}

#[test]
fn test_eight_ports() {
  type Ports = (i32, i32, i32, i32, i32, i32, i32, i32);
  assert_eq!(<Ports as PortList>::LEN, 8);

  type First = <Ports as GetPort<0>>::Type;
  type Last = <Ports as GetPort<7>>::Type;

  let _: First = 0i32;
  let _: Last = 7i32;
}

#[test]
fn test_nine_ports() {
  type Ports = (i32, i32, i32, i32, i32, i32, i32, i32, i32);
  assert_eq!(<Ports as PortList>::LEN, 9);

  type First = <Ports as GetPort<0>>::Type;
  type Last = <Ports as GetPort<8>>::Type;

  let _: First = 0i32;
  let _: Last = 8i32;
}

#[test]
fn test_ten_ports() {
  type Ports = (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32);
  assert_eq!(<Ports as PortList>::LEN, 10);

  type First = <Ports as GetPort<0>>::Type;
  type Last = <Ports as GetPort<9>>::Type;

  let _: First = 0i32;
  let _: Last = 9i32;
}

#[test]
fn test_eleven_ports() {
  type Ports = (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32);
  assert_eq!(<Ports as PortList>::LEN, 11);

  type First = <Ports as GetPort<0>>::Type;
  type Last = <Ports as GetPort<10>>::Type;

  let _: First = 0i32;
  let _: Last = 10i32;
}

#[test]
fn test_port_extraction_all_positions() {
  type Ports = (
    i32,
    String,
    bool,
    f64,
    u8,
    i16,
    u16,
    i32,
    u32,
    i64,
    u64,
    f32,
  );
  assert_eq!(<Ports as PortList>::LEN, 12);

  // Test all positions compile correctly
  let _: <Ports as GetPort<0>>::Type = 0i32;
  let _: <Ports as GetPort<1>>::Type = String::new();
  let _: <Ports as GetPort<2>>::Type = false;
  let _: <Ports as GetPort<3>>::Type = 0.0f64;
  let _: <Ports as GetPort<4>>::Type = 0u8;
  let _: <Ports as GetPort<5>>::Type = 0i16;
  let _: <Ports as GetPort<6>>::Type = 0u16;
  let _: <Ports as GetPort<7>>::Type = 0i32;
  let _: <Ports as GetPort<8>>::Type = 0u32;
  let _: <Ports as GetPort<9>>::Type = 0i64;
  let _: <Ports as GetPort<10>>::Type = 0u64;
  let _: <Ports as GetPort<11>>::Type = 0.0f32;
}

#[test]
fn test_single_port_type_alias() {
  type MyPort = SinglePort<String>;
  assert_eq!(<MyPort as PortList>::LEN, 1);

  type PortType = <MyPort as GetPort<0>>::Type;
  let _: PortType = "test".to_string();

  // Verify it's equivalent to (String,)
  type DirectPort = (String,);
  let _: <MyPort as GetPort<0>>::Type = <DirectPort as GetPort<0>>::Type::default();
}
