#![doc = include_str!("../README.md")]

pub mod consumers;
pub mod producers;

pub use consumers::EnvVarConsumer;
pub use producers::EnvVarProducer;
