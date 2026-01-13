//! # Arithmetic Nodes
//!
//! This module provides arithmetic operation nodes for performing mathematical operations
//! on numeric values.
//!
//! ## Standard Port Pattern
//!
//! All arithmetic nodes follow the standard port pattern for consistency:
//!
//! - **Input Ports**: `configuration` (currently unused, reserved for future use), plus data input ports
//! - **Output Ports**: Data output ports (`out`), plus `error`
//!
//! ## Available Nodes
//!
//! - **AddNode**: Addition operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **SubtractNode**: Subtraction operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **MultiplyNode**: Multiplication operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **DivideNode**: Division operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **ModuloNode**: Modulo operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **PowerNode**: Exponentiation operation (`configuration`, `base`, `exponent` → `out`, `error`)

pub mod add_node;
pub mod common;
pub mod divide_node;
pub mod modulo_node;
pub mod multiply_node;
pub mod power_node;
pub mod subtract_node;

#[cfg(test)]
mod add_node_test;
#[cfg(test)]
mod common_test;
#[cfg(test)]
mod divide_node_test;
#[cfg(test)]
mod modulo_node_test;
#[cfg(test)]
mod multiply_node_test;
#[cfg(test)]
mod power_node_test;
#[cfg(test)]
mod subtract_node_test;

pub use add_node::AddNode;
pub use divide_node::DivideNode;
pub use modulo_node::ModuloNode;
pub use multiply_node::MultiplyNode;
pub use power_node::PowerNode;
pub use subtract_node::SubtractNode;
