//! PostgreSQL producer and consumer for StreamWeave

pub mod consumers;
pub mod producers;

pub use consumers::PostgresConsumer;
pub use producers::PostgresProducer;
pub use streamweave_database::{
  DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseRow, DatabaseType,
};
