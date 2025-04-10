//! Resource management utilities for the effect system.
//!
//! This crate provides three main resource management patterns:
//! - `Bracket`: A three-phase pattern for resource acquisition, use, and release
//! - `Guard`: RAII-style wrapper for automatic resource cleanup
//! - `Scope`: Manager for multiple resources that need to be released together
//!
//! # Examples
//!
//! ```rust
//! use effect_resource::{Bracket, Guard, Scope};
//! use effect_core::effect::Effect;
//! use std::io::Error;
//!
//! // Using Bracket
//! let effect = Effect::<(), Error>::bracket(
//!     || Effect::pure(()), // acquire
//!     |_| Effect::pure(()), // use
//!     |_| Effect::pure(()), // release
//! );
//!
//! // Using Guard
//! let guard: Guard<i32, Error> = Guard::new(42, |_| Effect::pure(()));
//! let value = *guard; // Access the resource
//!
//! // Using Scope
//! let scope: Scope<Error> = Scope::new();
//! scope.add_resource(|| Effect::pure(()));
//! let effect = scope.run(|| Effect::pure(42));
//! ```

pub mod bracket;
pub mod guard;
pub mod scope;

pub use bracket::*;
pub use guard::*;
pub use scope::*;
