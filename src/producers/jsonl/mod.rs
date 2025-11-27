//! JSONL producer module.
//!
//! This module provides the `JsonlProducer` which reads JSON Lines (JSONL) files
//! and produces a stream of deserialized items.

/// The JSONL producer implementation.
pub mod jsonl_producer;
/// Output type definitions for the JSONL producer.
pub mod output;
/// Producer trait implementation for JSONL.
pub mod producer;
