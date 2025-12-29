#![doc = include_str!("../README.md")]

pub mod consumers;
pub mod producers;

pub use consumers::KafkaProducerConfig;
pub use producers::KafkaConsumerConfig;
