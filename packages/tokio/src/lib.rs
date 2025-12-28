//! Tokio channel integration for StreamWeave
//!
//! This package provides producers and consumers for integrating StreamWeave
//! with tokio channels (`tokio::sync::mpsc`).
//!
//! ## Producer
//!
//! The `ChannelProducer` reads items from a `tokio::sync::mpsc::Receiver`
//! and produces them into a StreamWeave stream.
//!
//! ## Consumer
//!
//! The `ChannelConsumer` sends items from a StreamWeave stream to a
//! `tokio::sync::mpsc::Sender`.

pub mod consumers;
pub mod producers;

pub use consumers::*;
pub use producers::*;
