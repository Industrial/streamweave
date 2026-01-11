//! # Producer Implementations
//!
//! This module provides producer implementations for StreamWeave, enabling data
//! to be read from various sources including arrays, files, databases, message
//! queues, network connections, and more.
//!
//! # Overview
//!
//! Producers are the source of data in StreamWeave pipelines. They implement
//! the [`crate::Producer`] trait to generate streams of data that can be transformed
//! and consumed by other components. This module provides a comprehensive set
//! of producer implementations for common data sources.
//!
//! # Key Concepts
//!
//! - **Producer Trait**: All producers implement the [`crate::Producer`] trait
//! - **Stream Generation**: Producers generate async streams of data items
//! - **Error Handling**: Producers support standard error handling strategies
//! - **Configuration**: Producers support configuration via [`crate::ProducerConfig`]
//!
//! # Producer Categories
//!
//! ## Memory-Based Producers
//!
//! - **[`ArrayProducer`]**: Produces items from arrays or slices
//! - **[`VecProducer`]**: Produces items from vectors
//! - **[`RangeProducer`]**: Produces numeric ranges
//! - **[`TokioChannelProducer`]**: Produces items from Tokio channels
//!
//! ## File System Producers
//!
//! - **[`FsFileProducer`]**: Reads data from files
//! - **[`FsDirectoryProducer`]**: Produces file paths from directory listings
//! - **[`CsvProducer`]**: Reads CSV files
//! - **[`JsonProducer`]**: Reads JSON files
//! - **[`JsonlProducer`]**: Reads JSON Lines files
//! - **[`ParquetProducer`]**: Reads Parquet files
//!
//! ## Database Producers
//!
//! - **[`DbMysqlProducer`]**: Reads data from MySQL databases
//! - **[`DbPostgresProducer`]**: Reads data from PostgreSQL databases
//! - **[`DbSqliteProducer`]**: Reads data from SQLite databases
//!
//! ## Network Producers
//!
//! - **[`TcpProducer`]**: Reads data from TCP connections
//! - **[`KafkaProducer`]**: Reads messages from Apache Kafka topics
//! - **[`RedisProducer`]**: Reads messages from Redis Streams
//!
//! ## System Integration Producers
//!
//! - **[`CommandProducer`]**: Executes commands and produces their output
//! - **[`ProcessProducer`]**: Produces data from external processes
//! - **[`StdioStdinProducer`]**: Reads from standard input
//! - **[`EnvVarProducer`]**: Produces environment variables
//! - **[`SignalProducer`]**: Produces Unix signals
//! - **[`TimerIntervalProducer`]**: Produces items at regular intervals
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::ArrayProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer from an array
//! let mut producer = ArrayProducer::new([1, 2, 3, 4, 5]);
//!
//! // Generate the stream
//! let mut stream = producer.produce();
//!
//! // Consume items from the stream
//! while let Some(item) = stream.next().await {
//!     println!("Item: {}", item);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## File-Based Producer
//!
//! ```rust,no_run
//! use streamweave::producers::FsFileProducer;
//! use futures::StreamExt;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads from a file
//! let mut producer = FsFileProducer::new(PathBuf::from("data.txt"));
//!
//! // Generate the stream (lines from the file)
//! let mut stream = producer.produce();
//!
//! // Process lines
//! while let Some(line) = stream.next().await {
//!     println!("Line: {}", line);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Database Producer
//!
//! ```rust,no_run
//! use streamweave::producers::DbPostgresProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads from PostgreSQL
//! let mut producer = DbPostgresProducer::new(
//!     "postgresql://user:password@localhost/database".to_string(),
//!     "SELECT id, name FROM users".to_string(),
//! ).await?;
//!
//! // Generate the stream (rows from the query)
//! let mut stream = producer.produce();
//!
//! // Process rows
//! while let Some(row) = stream.next().await {
//!     println!("Row: {:?}", row);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Message Queue Producer
//!
//! ```rust,no_run
//! use streamweave::producers::KafkaProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads from Kafka
//! let mut producer = KafkaProducer::new(
//!     "localhost:9092".to_string(),
//!     "my-topic".to_string(),
//!     "my-consumer-group".to_string(),
//! ).await?;
//!
//! // Generate the stream (messages from Kafka)
//! let mut stream = producer.produce();
//!
//! // Process messages
//! while let Some(message) = stream.next().await {
//!     println!("Message: {:?}", message);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! ## Async-First Design
//!
//! All producers generate async streams, enabling efficient I/O operations and
//! concurrent processing. This design allows producers to handle network operations,
//! file I/O, and database queries without blocking the async runtime.
//!
//! ## Error Handling
//!
//! Producers support the standard StreamWeave error handling strategies, allowing
//! you to configure how errors are handled (stop, skip, retry, custom). This provides
//! flexibility in handling transient failures and errors specific to different data sources.
//!
//! ## Resource Management
//!
//! Producers manage their own resources (file handles, database connections, network
//! connections) and clean them up when the producer is dropped. This ensures proper
//! resource cleanup and prevents resource leaks.
//!
//! ## Configuration
//!
//! All producers support configuration via [`crate::ProducerConfig`], which allows setting
//! error handling strategies, naming, and other producer-specific options. This provides
//! a consistent interface for configuring all producers.
//!
//! # Integration with StreamWeave
//!
//! Producers integrate seamlessly with StreamWeave's pipeline and graph systems:
//!
//! - **Pipeline API**: Use producers directly in pipelines
//! - **Graph API**: Wrap producers in [`crate::graph::nodes::ProducerNode`] for graph-based execution
//! - **Message Model**: Producer output can be wrapped in `Message<T>` for traceability
//! - **Transformers**: Producer output can be transformed using transformers
//! - **Consumers**: Producer output can be consumed by consumers
//!
//! # Common Patterns
//!
//! ## Reading from Multiple Sources
//!
//! Use multiple producers and merge their streams using transformers or graph nodes:
//!
//! ```rust,no_run
//! use streamweave::producers::{ArrayProducer, VecProducer};
//! use streamweave::transformers::MergeTransformer;
//!
//! // Create multiple producers
//! let producer1 = ArrayProducer::new([1, 2, 3]);
//! let producer2 = VecProducer::new(vec![4, 5, 6]);
//!
//! // Merge their streams (example using merge transformer)
//! // ...
//! ```
//!
//! ## Error Handling in Producers
//!
//! Configure error handling strategies for producers:
//!
//! ```rust
//! use streamweave::producers::ArrayProducer;
//! use streamweave::ErrorStrategy;
//!
//! let producer = ArrayProducer::new([1, 2, 3])
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("array-source".to_string());
//! ```
//!
//! ## Resource Cleanup
//!
//! Producers automatically clean up resources when dropped. For long-running producers,
//! ensure they are properly dropped to release resources:
//!
//! ```rust
//! use streamweave::producers::FsFileProducer;
//! use std::path::PathBuf;
//!
//! {
//!     let producer = FsFileProducer::new(PathBuf::from("data.txt"));
//!     // Producer is used here
//! } // Producer is dropped here, file handle is closed
//! ```

pub mod array_producer;
pub mod command_producer;
pub mod csv_producer;
pub mod db_mysql_producer;
pub mod db_postgres_producer;
pub mod db_sqlite_producer;
pub mod env_var_producer;
pub mod fs_directory_producer;
pub mod fs_file_producer;
pub mod json_producer;
pub mod jsonl_producer;
pub mod kafka_producer;
pub mod parquet_producer;
pub mod process_producer;
pub mod range_producer;
pub mod redis_producer;
pub mod signal_producer;
pub mod stdio_stdin_producer;
pub mod tcp_producer;
pub mod timer_interval_producer;
pub mod tokio_channel_producer;
pub mod vec_producer;

pub use array_producer::*;
pub use command_producer::*;
pub use csv_producer::*;
pub use db_mysql_producer::*;
pub use db_postgres_producer::*;
pub use db_sqlite_producer::*;
pub use env_var_producer::*;
pub use fs_directory_producer::*;
pub use fs_file_producer::*;
pub use json_producer::*;
pub use jsonl_producer::*;
pub use kafka_producer::*;
pub use parquet_producer::*;
pub use process_producer::*;
pub use range_producer::*;
pub use redis_producer::*;
pub use signal_producer::*;
pub use stdio_stdin_producer::*;
pub use tcp_producer::*;
pub use timer_interval_producer::*;
pub use tokio_channel_producer::*;
pub use vec_producer::*;

// Import types for rustdoc links
#[allow(unused_imports)]
use crate::graph::nodes::ProducerNode;
#[allow(unused_imports)]
use crate::producer::{Producer, ProducerConfig};
