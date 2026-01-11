//! # TCP Server Module for StreamWeave Graphs
//!
//! This module provides TCP server functionality for graph-based processing,
//! enabling StreamWeave graphs to accept incoming TCP connections and process
//! data through graph pipelines.
//!
//! ## Overview
//!
//! The TCP Server module provides:
//!
//! - **TCP Server**: High-level API for creating TCP servers that process connections through graphs
//! - **Connection Handling**: Manages incoming TCP connections and routes data through graphs
//! - **Graph Integration**: Integrates TCP server functionality with StreamWeave graph execution
//! - **Configurable Reading**: Supports line-based or delimiter-based data reading
//!
//! ## Core Components
//!
//! - **[`TcpGraphServer`]**: High-level TCP server that processes connections through graphs
//! - **[`TcpGraphServerConfig`]**: Configuration for TCP server behavior
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::graph::tcp_server::{TcpGraphServer, TcpGraphServerConfig};
//! use streamweave::graph::Graph;
//!
//! let config = TcpGraphServerConfig::default()
//!     .with_bind_address("127.0.0.1:8080");
//! let server = TcpGraphServer::new(config, graph);
//! server.start().await?;
//! ```

pub mod server;

pub use server::*;
