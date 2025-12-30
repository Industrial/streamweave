//! Integration tests for README examples

// Note: The README examples are mostly conceptual and show trait usage patterns.
// Since streamweave is a core trait library, most examples would require
// concrete implementations from other packages (like streamweave-array, etc.).
// These tests verify that the core traits can be used as documented.

use streamweave::{Consumer, Input, Output, Producer, Transformer};

// Verify that the traits exist and can be imported as shown in README
#[test]
fn test_trait_imports() {
  // This test verifies that all traits mentioned in README can be imported
  // The actual usage examples require concrete implementations from other packages
  fn _verify_traits_exist<P, T, C>()
  where
    P: Producer,
    P::Output: std::fmt::Debug + Clone + Send + Sync,
    T: Transformer,
    T::Input: std::fmt::Debug + Clone + Send + Sync,
    C: Consumer,
    C::Input: std::fmt::Debug + Clone + Send + Sync,
  {
    // Traits exist and can be used
  }
}

// Verify Producer trait structure matches README documentation
#[test]
fn test_producer_trait_structure() {
  use streamweave::Producer;

  // Verify Producer requires Output trait
  fn _requires_output<P>()
  where
    P: Producer + Output,
    P::Output: std::fmt::Debug + Clone + Send + Sync,
  {
  }

  // Verify Producer has OutputPorts associated type
  fn _has_output_ports<P: Producer>()
  where
    P::Output: std::fmt::Debug + Clone + Send + Sync,
  {
    let _: P::OutputPorts;
  }
}

// Verify Transformer trait structure matches README documentation
#[test]
fn test_transformer_trait_structure() {
  use streamweave::Transformer;

  // Verify Transformer requires both Input and Output traits
  fn _requires_input_output<T>()
  where
    T: Transformer + Input + Output,
    T::Input: std::fmt::Debug + Clone + Send + Sync,
  {
  }

  // Verify Transformer has InputPorts and OutputPorts associated types
  fn _has_ports<T: Transformer>()
  where
    T::Input: std::fmt::Debug + Clone + Send + Sync,
  {
    let _: T::InputPorts;
    let _: T::OutputPorts;
  }
}

// Verify Consumer trait structure matches README documentation
#[test]
fn test_consumer_trait_structure() {
  use streamweave::Consumer;

  // Verify Consumer requires Input trait
  fn _requires_input<C>()
  where
    C: Consumer + Input,
    C::Input: std::fmt::Debug + Clone + Send + Sync,
  {
  }

  // Verify Consumer has InputPorts associated type
  fn _has_input_ports<C: Consumer>()
  where
    C::Input: std::fmt::Debug + Clone + Send + Sync,
  {
    let _: C::InputPorts;
  }
}

// Verify Input and Output traits exist as documented
#[test]
fn test_input_output_traits() {
  use streamweave::{Input, Output};

  fn _verify_input<I: Input>() {
    let _: I::Input;
    let _: I::InputStream;
  }

  fn _verify_output<O: Output>() {
    let _: O::Output;
    let _: O::OutputStream;
  }
}

// Verify port system exists as documented
#[test]
fn test_port_system() {
  use streamweave::port::{GetPort, PortList};

  // Verify PortList trait exists
  type SinglePort = (i32,);
  let _len = <SinglePort as PortList>::LEN;
  assert_eq!(_len, 1);

  // Verify GetPort trait exists
  type PortType = <SinglePort as GetPort<0>>::Type;
  let _: PortType = 42i32;
}

// Verify configuration types exist as documented
#[test]
fn test_configuration_types() {
  use streamweave::{ConsumerConfig, ProducerConfig, TransformerConfig};

  // Verify config types can be created
  let _producer_config = ProducerConfig::<i32>::default();
  let _transformer_config = TransformerConfig::<i32>::default();
  let _consumer_config = ConsumerConfig::<i32>::default();
}
