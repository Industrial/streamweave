// Core consumers (WASM-compatible)
pub mod array;
pub mod channel;
pub mod hash_map;
pub mod hash_set;
pub mod string;
pub mod vec;

// Native-only consumers (require OS features)
#[cfg(not(target_arch = "wasm32"))]
pub mod command;
#[cfg(not(target_arch = "wasm32"))]
pub mod console;
#[cfg(not(target_arch = "wasm32"))]
pub mod file;

// File format consumers (native only, require file I/O)
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
