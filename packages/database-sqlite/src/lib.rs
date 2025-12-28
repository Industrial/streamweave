//! SQLite producer and consumer for StreamWeave

pub mod consumers;
pub mod producers;

pub use consumers::SqliteConsumer;
pub use producers::SqliteProducer;
pub use streamweave_database::{
  DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseRow, DatabaseType,
};
