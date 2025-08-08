//! Provides marker traits for thread safety and clonability in effect-core abstractions.
//!
//! These traits are used as bounds throughout the library to ensure that types are safe to use in concurrent and functional contexts.

/// Marker trait for types that are safe to share between threads.
///
/// This trait is automatically implemented for all types that are `Send + Sync + 'static`.
/// It is used as a bound for thread safety in effect-core abstractions.
///
/// # Examples
/// ```
/// use effect_core::types::threadsafe::ThreadSafe;
/// fn assert_thread_safe<T: ThreadSafe>() {}
/// assert_thread_safe::<i32>();
/// ```
pub trait ThreadSafe: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> ThreadSafe for T {}

/// Marker trait for types that are both `Clone` and [`ThreadSafe`].
///
/// This trait is automatically implemented for all types that are `Clone + ThreadSafe`.
/// It is used as a bound for clonable, thread-safe types in effect-core abstractions.
///
/// # Examples
/// ```
/// use effect_core::types::threadsafe::CloneableThreadSafe;
/// fn assert_cloneable_thread_safe<T: CloneableThreadSafe>() {}
/// assert_cloneable_thread_safe::<String>();
/// ```
pub trait CloneableThreadSafe: Clone + ThreadSafe {}
impl<T: Clone + ThreadSafe> CloneableThreadSafe for T {}

/// Marker trait for types that can be safely shared and mutated across threads.
///
/// This trait is automatically implemented for all types that are `Send + Sync + 'static`.
/// It is used as a bound for mutable thread-safe types in effect-core abstractions.
///
/// # Examples
/// ```
/// use effect_core::types::threadsafe::MutableThreadSafe;
/// fn assert_mutable_thread_safe<T: MutableThreadSafe>() {}
/// assert_mutable_thread_safe::<std::sync::Mutex<i32>>();
/// ```
pub trait MutableThreadSafe: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> MutableThreadSafe for T {}

/// Marker trait for types that are both `Clone` and [`MutableThreadSafe`].
///
/// This trait is automatically implemented for all types that are `Clone + MutableThreadSafe`.
/// It is used as a bound for clonable, mutable thread-safe types in effect-core abstractions.
///
/// # Examples
/// ```
/// use effect_core::types::threadsafe::CloneableMutableThreadSafe;
/// fn assert_cloneable_mutable_thread_safe<T: CloneableMutableThreadSafe>() {}
/// assert_cloneable_mutable_thread_safe::<std::sync::Arc<std::sync::Mutex<i32>>>();
/// ```
pub trait CloneableMutableThreadSafe: Clone + MutableThreadSafe {}
impl<T: Clone + MutableThreadSafe> CloneableMutableThreadSafe for T {}
