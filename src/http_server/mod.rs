//! HTTP Server Integration for StreamWeave Graphs
//!
//! This module provides HTTP server integration for StreamWeave graphs, enabling
//! building HTTP APIs using StreamWeave pipelines and graphs.

pub mod error;
pub mod middleware;
pub mod nodes;
pub mod server;
pub mod types;

pub use crate::error::{ErrorAction, ErrorContext, ErrorStrategy, StreamError};
pub use error::*;
pub use middleware::*;
pub use nodes::*;
pub use server::*;
pub use types::*;
