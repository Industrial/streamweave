// Core producers
pub mod array;
pub mod channel;
pub mod hash_map;
pub mod hash_set;
pub mod range;
pub mod string;
pub mod vec;

// Producers requiring tokio runtime features
#[cfg(feature = "native")]
pub mod interval;

// Producers requiring random number generation
#[cfg(feature = "random")]
pub mod random_number;

// Native-only producers (require OS features and tokio runtime features)
#[cfg(feature = "native")]
pub mod command;
#[cfg(feature = "native")]
pub mod env_var;
/// Producer that reads lines from a file.
#[cfg(feature = "native")]
pub mod file;

// File format producers (native only, require file I/O)
#[cfg(feature = "file-formats")]
pub mod csv;
#[cfg(feature = "file-formats")]
pub mod jsonl;
#[cfg(feature = "file-formats")]
pub mod msgpack;
#[cfg(feature = "file-formats")]
pub mod parquet;

// Kafka integration (native only, requires librdkafka and cmake)
#[cfg(feature = "kafka")]
pub mod kafka;

// Redis Streams integration (native only, requires Redis)
#[cfg(feature = "redis")]
pub mod redis;

// Database integration (native only, requires PostgreSQL/MySQL)
#[cfg(feature = "database")]
pub mod database;

// HTTP polling integration (native only)
#[cfg(feature = "http-poll")]
pub mod http_poll;
