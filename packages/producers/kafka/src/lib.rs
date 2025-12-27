//! Kafka producer for StreamWeave

pub mod kafka_producer;
pub mod output;
pub mod producer;

pub use kafka_producer::*;
// pub use output::*;  // Unused - output trait is in streamweave-core
// pub use producer::*;  // Unused - producer trait is in streamweave-core
