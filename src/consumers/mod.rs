//! Consumer implementations for StreamWeave pipelines.
//!
//! This module provides various consumer implementations for different output
//! destinations and use cases. Consumers are the final stage of a StreamWeave
//! pipeline, responsible for handling stream items (writing, sending, storing, etc.).
//!
//! # Overview
//!
//! Consumers implement the [`crate::Consumer`] trait and can be used in any StreamWeave
//! pipeline to handle stream output. This module provides implementations for:
//!
//! - **File System**: File, directory, and CSV consumers
//! - **Databases**: MySQL, PostgreSQL, and SQLite consumers
//! - **Message Queues**: Kafka and Redis consumers
//! - **Networks**: TCP and HTTP consumers
//! - **Standard I/O**: Console, stdout, and stderr consumers
//! - **In-Memory**: Vector and channel consumers
//! - **Data Formats**: JSONL and Parquet consumers
//! - **Error Handling**: Dead letter queue consumer
//!
//! # Core Concepts
//!
//! - **Consumer Trait**: All consumers implement the [`crate::Consumer`] trait
//! - **Error Handling**: Configurable error strategies (stop, skip, retry, custom)
//! - **Configuration**: Standard configuration via [`ConsumerConfig`]
//! - **Type Safety**: Generic type parameters ensure type safety throughout pipelines
//!
//! # Quick Start
//!
//! ## File Output
//!
//! ```rust
//! use streamweave::consumers::FsFileConsumer;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut consumer = FsFileConsumer::new("/tmp/output.txt".to_string());
//! let stream = stream::iter(vec!["line1\n".to_string(), "line2\n".to_string()]);
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## Database Output
//!
//! ```rust
//! use streamweave::consumers::DbPostgresConsumer;
//! use streamweave::db::DatabaseConsumerConfig;
//!
//! let db_config = DatabaseConsumerConfig {
//!     connection_string: "postgres://localhost/db".to_string(),
//!     table_name: "records".to_string(),
//!     // ... other configuration
//! };
//! let consumer = DbPostgresConsumer::new(db_config);
//! ```
//!
//! ## Message Queue Output
//!
//! ```rust
//! use streamweave::consumers::{KafkaConsumer, KafkaProducerConfig};
//!
//! # use serde::Serialize;
//! # #[derive(Serialize, Clone, Debug)]
//! # struct Event { id: u32 }
//! let kafka_config = KafkaProducerConfig::default()
//!     .with_bootstrap_servers("localhost:9092")
//!     .with_topic("events");
//! let consumer = KafkaConsumer::<Event>::new(kafka_config);
//! ```
//!
//! # Available Consumers
//!
//! ## File System Consumers
//!
//! - [`FsFileConsumer`]: Write items to files
//! - [`FsDirectoryConsumer`]: Create directories from stream paths
//! - [`CsvConsumer`]: Write items to CSV files
//!
//! ## Database Consumers
//!
//! - [`DbMysqlConsumer`]: Write rows to MySQL tables
//! - [`DbPostgresConsumer`]: Write rows to PostgreSQL tables
//! - [`DbSqliteConsumer`]: Write rows to SQLite tables
//!
//! ## Message Queue Consumers
//!
//! - [`KafkaConsumer`]: Publish messages to Kafka topics
//! - [`RedisConsumer`]: Write items to Redis Streams
//!
//! ## Network Consumers
//!
//! - [`TcpConsumer`]: Send items over TCP connections
//! - [`ProcessConsumer`]: Send items to external process stdin
//! - [`CommandConsumer`]: Execute commands for each item
//!
//! ## Standard I/O Consumers
//!
//! - [`StdioStdoutConsumer`]: Write items to stdout
//! - [`StdioStderrConsumer`]: Write items to stderr
//! - [`ConsoleConsumer`]: Print items to console
//!
//! ## In-Memory Consumers
//!
//! - [`VecConsumer`]: Collect items into a Vec
//! - [`TokioChannelConsumer`]: Forward items to Tokio channels
//!
//! ## Data Format Consumers
//!
//! - [`JsonlConsumer`]: Write items to JSON Lines files
//! - [`ParquetConsumer`]: Write Arrow record batches to Parquet files
//!
//! ## Error Handling Consumers
//!
//! - [`DeadLetterQueue`]: Collect failed items for analysis
//!
//! # Integration with StreamWeave
//!
//! All consumers in this module implement the [`crate::Consumer`] trait and can be used
//! with the [`crate::Pipeline`] API or the [`crate::Graph`] API. They support the standard
//! error handling strategies and configuration options provided by [`crate::ConsumerConfig`].

// Import types for rustdoc intra-doc links
#[allow(unused_imports)]
use crate::consumer::{Consumer, ConsumerConfig};
#[allow(unused_imports)]
use crate::graph::Graph;
#[allow(unused_imports)]
use crate::pipeline::Pipeline;

pub mod array_consumer;
pub mod command_consumer;
pub mod console_consumer;
pub mod csv_consumer;
pub mod db_mysql_consumer;
pub mod db_postgres_consumer;
pub mod db_sqlite_consumer;
pub mod dead_letter_queue;
pub mod env_var_consumer;
pub mod fs_directory_consumer;
pub mod fs_file_consumer;
pub mod json_consumer;
pub mod jsonl_consumer;
pub mod kafka_consumer;
pub mod parquet_consumer;
pub mod process_consumer;
pub mod redis_consumer;
pub mod stdio_stderr_consumer;
pub mod stdio_stdout_consumer;
pub mod tcp_consumer;
pub mod tokio_channel_consumer;
pub mod vec_consumer;

pub use array_consumer::*;
pub use command_consumer::*;
pub use console_consumer::*;
pub use csv_consumer::*;
pub use db_mysql_consumer::*;
pub use db_postgres_consumer::*;
pub use db_sqlite_consumer::*;
pub use dead_letter_queue::*;
pub use env_var_consumer::*;
pub use fs_directory_consumer::*;
pub use fs_file_consumer::*;
// pub use json_consumer::*; // TODO: JsonConsumer struct not yet implemented, file only contains tests
pub use jsonl_consumer::*;
pub use kafka_consumer::*;
pub use parquet_consumer::*;
pub use process_consumer::*;
pub use redis_consumer::*;
pub use stdio_stderr_consumer::*;
pub use stdio_stdout_consumer::*;
pub use tcp_consumer::*;
pub use tokio_channel_consumer::*;
pub use vec_consumer::*;
