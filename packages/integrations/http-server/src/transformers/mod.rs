//! # HTTP Server Transformers
//!
//! Transformers for HTTP request processing in graph-based servers.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod path_router;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
mod path_router_input;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
mod path_router_output;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub use path_router::{PathBasedRouterTransformer, PathRouterConfig, RoutePattern};
