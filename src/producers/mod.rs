// Core producers (WASM-compatible)
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

// Native-only producers (require OS features)
#[cfg(not(target_arch = "wasm32"))]
pub mod command;
#[cfg(not(target_arch = "wasm32"))]
pub mod env_var;
#[cfg(not(target_arch = "wasm32"))]
pub mod file;

// File format producers (native only, require file I/O)
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub mod csv;
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub mod jsonl;
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub mod msgpack;
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub mod parquet;

// Kafka integration (native only, requires librdkafka and cmake)
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
pub mod kafka;

// Redis Streams integration (native only, requires Redis)
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
pub mod redis_streams;

// Database integration (native only, requires PostgreSQL/MySQL)
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
pub mod database;
