//! HTTP poll producer module.
//!
//! This module provides the `HttpPollProducer` which periodically polls an HTTP endpoint
//! and produces a stream of responses.

/// The HTTP poll producer implementation.
pub mod http_poll_producer;
/// Input type definitions for the HTTP poll producer.
pub mod input;
/// Output type definitions for the HTTP poll producer.
pub mod output;
/// Producer trait implementation for HTTP poll.
pub mod producer;
