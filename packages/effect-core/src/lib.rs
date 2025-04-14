//! Core types and traits for effect handling.
//!
//! This module provides the core types and traits for the effect system,
//! including the `Effect` type and the `Monad` trait.

pub mod applicative;
pub mod effect;
pub mod either;
pub mod filterable;
pub mod foldable;
pub mod functor;
pub mod monad;
pub mod non_empty_array;
pub mod non_empty_string;
pub mod option;
pub mod result;
pub mod scannable;
pub mod zippable;

pub use applicative::*;
pub use effect::*;
pub use either::*;
pub use filterable::*;
pub use foldable::*;
pub use functor::*;
pub use monad::*;
pub use non_empty_array::*;
pub use non_empty_string::*;
pub use option::*;
pub use result::*;
pub use scannable::*;
pub use zippable::*;
