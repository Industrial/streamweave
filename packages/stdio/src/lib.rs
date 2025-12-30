#![doc = include_str!("../README.md")]

pub mod stderr_consumer;
pub mod stdin_producer;
pub mod stdout_consumer;

pub use stderr_consumer::*;
pub use stdin_producer::*;
pub use stdout_consumer::*;
