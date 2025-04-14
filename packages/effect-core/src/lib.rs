//! Core types and traits for effect handling.
//!
//! This module provides the core types and traits for the effect system,
//! including the `Effect` type and the `Monad` trait.

pub mod applicative;
pub mod distinctable;
pub mod effect;
pub mod either;
pub mod filterable;
pub mod foldable;
pub mod functor;
pub mod groupable;
pub mod interleaveable;
pub mod monad;
pub mod non_empty_array;
pub mod non_empty_string;
pub mod option;
pub mod partitionable;
pub mod result;
pub mod scannable;
pub mod takeable;
pub mod windowable;
pub mod zippable;

pub use applicative::*;
pub use distinctable::*;
pub use effect::*;
pub use either::*;
pub use filterable::*;
pub use foldable::*;
pub use functor::*;
pub use groupable::*;
pub use interleaveable::*;
pub use monad::*;
pub use non_empty_array::*;
pub use non_empty_string::*;
pub use option::*;
pub use partitionable::*;
pub use result::*;
pub use scannable::*;
pub use takeable::*;
pub use windowable::*;
pub use zippable::*;
