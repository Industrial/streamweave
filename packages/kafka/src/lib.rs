#![doc = include_str!("../README.md")]

pub mod kafka_consumer;
pub mod kafka_producer;

pub use kafka_consumer::*;
pub use kafka_producer::*;
