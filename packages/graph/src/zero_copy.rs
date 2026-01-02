//! Zero-copy architecture traits and utilities.
//!
//! This module provides traits and utilities for implementing zero-copy
//! data sharing patterns in stream processing graphs. The primary focus
//! is on shared ownership using `Arc` for fan-out scenarios where one
//! item needs to be sent to multiple consumers without copying.
//!
//! ## Zero-Copy Transformations with Cow
//!
//! The `ZeroCopyTransformer` trait enables conditional cloning using `Cow`
//! (Clone on Write) to avoid unnecessary clones in transformer operations.
//! Transformers that may or may not need to own data can use `Cow` for
//! zero-copy when possible, cloning only when necessary.
//!
//! ## When to Use Cow
//!
//! - **Cow::Borrowed**: Use when you can transform data without owning it
//!   (e.g., read-only transformations, filtering, mapping that doesn't mutate)
//! - **Cow::Owned**: Use when transformation requires ownership or produces
//!   a new value (e.g., mapping that creates new data, aggregations)
//!
//! ## Cloning Behavior
//!
//! - `Cow::Borrowed` values are cloned only when the transformation requires
//!   mutation or when the output needs to outlive the input
//! - `Cow::Owned` values are already owned, so no additional cloning occurs
//! - The transformation itself determines whether cloning is necessary

use std::borrow::Cow;
use std::sync::{Arc, Mutex, Weak};
use streamweave::Transformer;

/// Trait for types that can be shared using zero-copy semantics.
///
/// This trait enables zero-copy sharing of data in fan-out scenarios where
/// one item needs to be sent to multiple consumers. Types implementing this
/// trait can be converted to a shared form (typically `Arc<T>`) and back
/// without unnecessary cloning.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
/// use streamweave_graph::ZeroCopyShare;
///
/// let data = vec![1, 2, 3];
/// let shared = data.to_shared();
/// let cloned = shared.clone(); // Zero-cost clone of Arc
/// let original = ZeroCopyShare::from_shared(shared);
/// ```
pub trait ZeroCopyShare: Send + Sync + 'static {
  /// The shared type used for zero-copy sharing.
  ///
  /// This is typically `Arc<Self>`, but can be customized for types
  /// that have more efficient sharing mechanisms.
  type Shared: Clone + Send + Sync;

  /// Converts `self` into a shared form for zero-copy sharing.
  ///
  /// This method wraps the value in an `Arc` (or similar shared container)
  /// to enable zero-copy sharing across multiple consumers.
  fn to_shared(self) -> Self::Shared;

  /// Converts a shared value back into an owned value.
  ///
  /// This method attempts to unwrap the shared container. If there are
  /// multiple references, it will clone the inner value.
  ///
  /// # Performance
  ///
  /// This is a zero-cost operation if there is only one reference to
  /// the shared value. Otherwise, it clones the inner value.
  fn from_shared(shared: Self::Shared) -> Self;
}

/// Blanket implementation of `ZeroCopyShare` for all `Send + Sync + 'static` types.
///
/// This provides a default implementation using `Arc<T>` for shared ownership.
/// Types that meet the bounds automatically get zero-copy sharing capabilities.
impl<T: Clone + Send + Sync + 'static> ZeroCopyShare for T {
  type Shared = Arc<T>;

  fn to_shared(self) -> Self::Shared {
    Arc::new(self)
  }

  fn from_shared(shared: Self::Shared) -> Self {
    // Try to unwrap the Arc first (zero-cost if only one reference)
    Arc::try_unwrap(shared).unwrap_or_else(|arc| (*arc).clone())
  }
}

/// Trait for types that support weak references in zero-copy sharing.
///
/// This trait extends `ZeroCopyShare` with weak reference support, enabling
/// more efficient memory management in scenarios where you want to avoid
/// keeping data alive unnecessarily.
///
/// Weak references allow you to check if the shared data is still available
/// without incrementing the reference count, which is useful for caching
/// and other scenarios where you want to avoid memory leaks.
pub trait ZeroCopyShareWeak: ZeroCopyShare {
  /// The weak reference type for this shared type.
  ///
  /// This is typically `Weak<T>` where `T` is the type being shared.
  type Weak: Send + Sync;

  /// Creates a weak reference to the shared value.
  ///
  /// Weak references do not keep the shared data alive, allowing it to be
  /// dropped when all strong references are gone.
  fn downgrade(shared: &Self::Shared) -> Self::Weak;

  /// Attempts to upgrade a weak reference to a strong reference.
  ///
  /// Returns `Some(Shared)` if the data is still alive, `None` otherwise.
  fn upgrade(weak: &Self::Weak) -> Option<Self::Shared>;
}

/// Blanket implementation of `ZeroCopyShareWeak` for all `Send + Sync + 'static` types.
///
/// This provides weak reference support using `Weak<T>` for types that
/// implement `ZeroCopyShare`.
impl<T: Clone + Send + Sync + 'static> ZeroCopyShareWeak for T {
  type Weak = Weak<T>;

  fn downgrade(shared: &Self::Shared) -> Self::Weak {
    Arc::downgrade(shared)
  }

  fn upgrade(weak: &Self::Weak) -> Option<Self::Shared> {
    weak.upgrade()
  }
}

/// A pool of pre-allocated `Arc` wrappers for high-performance scenarios.
///
/// This structure maintains a pool of reusable `Arc<T>` instances to reduce
/// allocation overhead in hot paths, particularly in fan-out scenarios where
/// many `Arc` wrappers are created and dropped frequently.
///
/// # Example
///
/// ```rust
/// use streamweave_graph::ArcPool;
///
/// let pool = ArcPool::<Vec<i32>>::new(10); // Pool with max size 10
/// let arc = pool.get(); // Get an Arc from pool or allocate new one
/// // Use arc...
/// pool.return_arc(arc); // Return to pool for reuse
/// ```
pub struct ArcPool<T: Clone + Send + Sync + 'static> {
  /// Pool of values (not `Arc<T>`) for reuse
  /// Values are wrapped in Arc when retrieved from the pool
  pool: Arc<Mutex<Vec<T>>>,
  /// Maximum number of values to keep in the pool
  max_size: usize,
  /// Number of times a value was successfully retrieved from the pool (hit)
  hits: Arc<std::sync::atomic::AtomicU64>,
  /// Number of times the pool was empty and a new value had to be allocated (miss)
  misses: Arc<std::sync::atomic::AtomicU64>,
  /// Number of times a value was successfully returned to the pool
  returns: Arc<std::sync::atomic::AtomicU64>,
  /// Number of times return_arc failed (Arc had multiple references or pool was full)
  return_failures: Arc<std::sync::atomic::AtomicU64>,
}

impl<T: Clone + Send + Sync + 'static> ArcPool<T> {
  /// Create a new `ArcPool` with the specified maximum size.
  ///
  /// The pool will maintain at most `max_size` `Arc` instances. When the pool
  /// is full, returned `Arc`s will be discarded rather than added to the pool.
  ///
  /// # Arguments
  ///
  /// * `max_size` - Maximum number of `Arc` instances to keep in the pool
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::ArcPool;
  ///
  /// let pool = ArcPool::<String>::new(100);
  /// ```
  #[must_use]
  pub fn new(max_size: usize) -> Self {
    Self {
      pool: Arc::new(Mutex::new(Vec::with_capacity(max_size.min(16)))), // Pre-allocate some capacity
      max_size,
      hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
      misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
      returns: Arc::new(std::sync::atomic::AtomicU64::new(0)),
      return_failures: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    }
  }

  /// Get an `Arc<T>` from the pool, or create a new one with the given value.
  ///
  /// This method tries to reuse a value from the pool. If the pool is empty,
  /// it uses the provided value. The value is wrapped in `Arc` before returning.
  ///
  /// # Arguments
  ///
  /// * `value` - The value to wrap in an `Arc` (used if pool is empty)
  ///
  /// # Returns
  ///
  /// An `Arc<T>` containing either a pooled value or the provided value
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::ArcPool;
  ///
  /// let pool = ArcPool::<String>::new(10);
  /// let arc1 = pool.get_or_create("hello".to_string());
  /// // Use arc1...
  /// pool.return_arc(arc1);
  /// // Later, get from pool
  /// let arc2 = pool.get_or_create("world".to_string()); // May reuse "hello" from pool
  /// ```
  pub fn get_or_create(&self, value: T) -> Arc<T> {
    let mut pool = self.pool.lock().unwrap();

    // Try to get a value from the pool
    if let Some(pooled_value) = pool.pop() {
      // Reuse pooled value (hit)
      self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
      Arc::new(pooled_value)
    } else {
      // Pool is empty, use provided value (miss)
      self
        .misses
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
      Arc::new(value)
    }
  }

  /// Return an `Arc<T>` to the pool for reuse.
  ///
  /// If the pool is not full and the `Arc` has only one reference, it will be
  /// unwrapped and the value stored for potential reuse. If the pool is full or
  /// the `Arc` has multiple references, it will be discarded.
  ///
  /// # Arguments
  ///
  /// * `arc` - The `Arc` to return to the pool
  ///
  /// # Returns
  ///
  /// `true` if the value was successfully returned to the pool, `false` otherwise
  ///
  /// # Note
  ///
  /// Only `Arc`s with a single reference can be returned to the pool. If the
  /// `Arc` has multiple references (e.g., in fan-out scenarios), it cannot be
  /// unwrapped and will be dropped when the last reference is dropped.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::ArcPool;
  /// use std::sync::Arc;
  ///
  /// let pool = ArcPool::<String>::new(10);
  /// let arc = pool.get_or_create("hello".to_string());
  /// // Use arc...
  /// let returned = pool.return_arc(arc); // Returns true if successful
  /// ```
  pub fn return_arc(&self, arc: Arc<T>) -> bool {
    // Try to unwrap the Arc - only succeeds if this is the only reference
    match Arc::try_unwrap(arc) {
      Ok(value) => {
        // Successfully unwrapped, can return to pool
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_size {
          pool.push(value);
          self
            .returns
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
          true
        } else {
          // Pool is full, drop the value
          self
            .return_failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
          false
        }
      }
      Err(_) => {
        // Arc has multiple references, cannot unwrap
        // The Arc will be dropped when the last reference is dropped
        self
          .return_failures
          .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        false
      }
    }
  }

  /// Get the current number of `Arc` instances in the pool.
  ///
  /// # Returns
  ///
  /// The number of `Arc` instances currently in the pool
  #[must_use]
  pub fn len(&self) -> usize {
    self.pool.lock().unwrap().len()
  }

  /// Check if the pool is empty.
  ///
  /// # Returns
  ///
  /// `true` if the pool contains no `Arc` instances, `false` otherwise
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.pool.lock().unwrap().is_empty()
  }

  /// Clear all values from the pool.
  ///
  /// This will drop all pooled values, freeing their memory.
  /// Statistics are not reset by this method.
  pub fn clear(&self) {
    self.pool.lock().unwrap().clear();
  }

  /// Get pool statistics.
  ///
  /// # Returns
  ///
  /// A `PoolStatistics` struct containing hit rate, pool size, and other metrics.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::ArcPool;
  ///
  /// let pool = ArcPool::<String>::new(10);
  /// let stats = pool.statistics();
  /// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
  /// ```
  pub fn statistics(&self) -> PoolStatistics {
    let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
    let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
    let returns = self.returns.load(std::sync::atomic::Ordering::Relaxed);
    let return_failures = self
      .return_failures
      .load(std::sync::atomic::Ordering::Relaxed);
    let pool_size = self.len();
    let total_requests = hits + misses;

    PoolStatistics {
      hits,
      misses,
      returns,
      return_failures,
      pool_size,
      max_size: self.max_size,
      total_requests,
    }
  }

  /// Get the number of pool hits (values reused from pool).
  ///
  /// # Returns
  ///
  /// The number of times a value was successfully retrieved from the pool.
  pub fn hits(&self) -> u64 {
    self.hits.load(std::sync::atomic::Ordering::Relaxed)
  }

  /// Get the number of pool misses (new allocations).
  ///
  /// # Returns
  ///
  /// The number of times the pool was empty and a new value had to be allocated.
  pub fn misses(&self) -> u64 {
    self.misses.load(std::sync::atomic::Ordering::Relaxed)
  }

  /// Get the number of successful returns to the pool.
  ///
  /// # Returns
  ///
  /// The number of times a value was successfully returned to the pool.
  pub fn returns(&self) -> u64 {
    self.returns.load(std::sync::atomic::Ordering::Relaxed)
  }

  /// Get the number of failed return attempts.
  ///
  /// # Returns
  ///
  /// The number of times `return_arc` failed (Arc had multiple references or pool was full).
  pub fn return_failures(&self) -> u64 {
    self
      .return_failures
      .load(std::sync::atomic::Ordering::Relaxed)
  }

  /// Reset all statistics counters.
  ///
  /// This resets hits, misses, returns, and return_failures to zero.
  /// The pool contents are not affected.
  pub fn reset_statistics(&self) {
    self.hits.store(0, std::sync::atomic::Ordering::Relaxed);
    self.misses.store(0, std::sync::atomic::Ordering::Relaxed);
    self.returns.store(0, std::sync::atomic::Ordering::Relaxed);
    self
      .return_failures
      .store(0, std::sync::atomic::Ordering::Relaxed);
  }
}

/// Statistics for an `ArcPool`.
///
/// This struct provides detailed metrics about pool usage and effectiveness.
#[derive(Debug, Clone)]
pub struct PoolStatistics {
  /// Number of pool hits (values reused from pool)
  pub hits: u64,
  /// Number of pool misses (new allocations)
  pub misses: u64,
  /// Number of successful returns to pool
  pub returns: u64,
  /// Number of failed return attempts
  pub return_failures: u64,
  /// Current number of values in the pool
  pub pool_size: usize,
  /// Maximum pool size
  pub max_size: usize,
  /// Total number of get_or_create requests
  pub total_requests: u64,
}

impl PoolStatistics {
  /// Calculate the hit rate (hits / total_requests).
  ///
  /// # Returns
  ///
  /// Hit rate as a value between 0.0 and 1.0, or 0.0 if no requests have been made.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::ArcPool;
  ///
  /// let pool = ArcPool::<String>::new(10);
  /// let stats = pool.statistics();
  /// if stats.hit_rate() > 0.5 {
  ///     println!("Pool is effective: {:.2}% hit rate", stats.hit_rate() * 100.0);
  /// }
  /// ```
  pub fn hit_rate(&self) -> f64 {
    if self.total_requests == 0 {
      return 0.0;
    }
    self.hits as f64 / self.total_requests as f64
  }

  /// Calculate the return success rate (returns / (returns + return_failures)).
  ///
  /// # Returns
  ///
  /// Return success rate as a value between 0.0 and 1.0, or 0.0 if no returns attempted.
  pub fn return_success_rate(&self) -> f64 {
    let total_returns = self.returns + self.return_failures;
    if total_returns == 0 {
      return 0.0;
    }
    self.returns as f64 / total_returns as f64
  }

  /// Get a formatted summary of the statistics.
  ///
  /// # Returns
  ///
  /// A string containing a human-readable summary of pool statistics.
  pub fn summary(&self) -> String {
    format!(
      "Pool Statistics:\n  Hits: {} ({:.2}%)\n  Misses: {}\n  Returns: {} ({:.2}%)\n  Return Failures: {}\n  Pool Size: {}/{}\n  Total Requests: {}",
      self.hits,
      self.hit_rate() * 100.0,
      self.misses,
      self.returns,
      self.return_success_rate() * 100.0,
      self.return_failures,
      self.pool_size,
      self.max_size,
      self.total_requests
    )
  }
}

impl<T: Clone + Send + Sync + 'static> Clone for ArcPool<T> {
  fn clone(&self) -> Self {
    Self {
      pool: Arc::clone(&self.pool),
      max_size: self.max_size,
      hits: Arc::clone(&self.hits),
      misses: Arc::clone(&self.misses),
      returns: Arc::clone(&self.returns),
      return_failures: Arc::clone(&self.return_failures),
    }
  }
}

/// Trait for transformers that support zero-copy transformations using `Cow`.
///
/// This trait extends the `Transformer` trait with a zero-copy transformation
/// method that uses `Cow` (Clone on Write) to avoid unnecessary clones. Transformers
/// implementing this trait can process data without cloning when the transformation
/// doesn't require ownership.
///
/// # Zero-Copy Semantics
///
/// - **Cow::Borrowed**: The input is borrowed, and the transformation can work
///   with a reference. No cloning occurs unless the transformation requires mutation.
/// - **Cow::Owned**: The input is owned, allowing in-place transformations or
///   transformations that consume the input.
///
/// # When Cloning Occurs
///
/// Cloning happens when:
/// - A `Cow::Borrowed` value needs to be mutated
/// - The output needs to outlive the input
/// - The transformation produces a new value (always returns `Cow::Owned`)
///
/// # Example
///
/// ```rust
/// use std::borrow::Cow;
/// use streamweave_graph::ZeroCopyTransformer;
/// use streamweave::Transformer;
///
/// struct IdentityTransformer;
///
/// impl Transformer for IdentityTransformer {
///     type Input = i32;
///     type Output = i32;
///     // ... other required methods
/// }
///
/// impl ZeroCopyTransformer for IdentityTransformer {
///     fn transform_zero_copy<'a>(&mut self, input: Cow<'a, i32>) -> Cow<'a, i32> {
///         // For identity, we can return the same Cow variant
///         input
///     }
/// }
/// ```
pub trait ZeroCopyTransformer: Transformer
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
  Self::Output: std::fmt::Debug + Clone + Send + Sync,
{
  /// Transforms an item using zero-copy semantics with `Cow`.
  ///
  /// This method allows transformers to avoid unnecessary clones by accepting
  /// data as `Cow` (Clone on Write). The transformer can work with borrowed
  /// data when possible, only cloning when necessary.
  ///
  /// # Arguments
  ///
  /// * `input` - The input item wrapped in `Cow`, which can be either
  ///   `Cow::Borrowed` (zero-copy) or `Cow::Owned` (already owned)
  ///
  /// # Returns
  ///
  /// The transformed item wrapped in `Cow`. Typically returns `Cow::Owned`
  /// since transformations usually produce new values, but can return
  /// `Cow::Borrowed` for identity or read-only transformations.
  ///
  /// # Lifetime
  ///
  /// The lifetime `'a` ensures that borrowed data outlives the transformation.
  /// When returning `Cow::Borrowed`, the output must not outlive the input.
  ///
  /// # Example
  ///
  /// ```rust
  /// use std::borrow::Cow;
  /// use streamweave_graph::ZeroCopyTransformer;
  ///
  /// // For a mapping transformation that creates new values
  /// fn transform_zero_copy<'a>(&mut self, input: Cow<'a, i32>) -> Cow<'a, i32> {
  ///     // Always return Owned since we're creating a new value
  ///     Cow::Owned(*input * 2)
  /// }
  ///
  /// // For an identity transformation
  /// fn transform_zero_copy<'a>(&mut self, input: Cow<'a, i32>) -> Cow<'a, i32> {
  ///     // Can return the same Cow variant (zero-copy)
  ///     input
  /// }
  /// ```
  fn transform_zero_copy<'a>(&mut self, input: Cow<'a, Self::Input>) -> Cow<'a, Self::Output>;
}

/// Trait for types that support in-place transformation without allocation.
///
/// This trait enables even more efficient zero-copy transformations for mutable types
/// that can be transformed in-place. Types implementing this trait can transform
/// themselves without creating new allocations, making it the most efficient
/// transformation method when ownership is available.
///
/// # Zero-Copy Semantics
///
/// - Takes `self` by value, allowing in-place mutation
/// - Returns `Self`, enabling zero-allocation transformations
/// - Most efficient when the transformation can modify the value directly
///
/// # When to Use
///
/// - Use `InPlaceTransform` when you have owned values and can mutate them
/// - Prefer over `ZeroCopyTransformer` when you don't need to preserve the original
/// - Ideal for transformations that modify existing data structures
///
/// # Example
///
/// ```rust
/// use streamweave_graph::InPlaceTransform;
///
/// struct VecDoubler;
///
/// impl InPlaceTransform for Vec<i32> {
///     fn transform_in_place(self) -> Self {
///         self.into_iter().map(|x| x * 2).collect()
///     }
/// }
///
/// let mut vec = vec![1, 2, 3];
/// vec = vec.transform_in_place(); // Transforms in-place
/// assert_eq!(vec, vec![2, 4, 6]);
/// ```
pub trait InPlaceTransform {
  /// Transforms `self` in-place, returning the transformed value.
  ///
  /// This method takes ownership of `self` and returns a transformed version.
  /// For types that can be mutated in-place, this allows zero-allocation
  /// transformations when ownership is available.
  ///
  /// # Returns
  ///
  /// The transformed value of type `Self`. This may be the same value
  /// (if mutated in-place) or a new value (if transformation requires
  /// reconstruction).
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::InPlaceTransform;
  ///
  /// // For a type that can be mutated in-place
  /// impl InPlaceTransform for Vec<i32> {
  ///     fn transform_in_place(mut self) -> Self {
  ///         for item in &mut self {
  ///             *item *= 2;
  ///         }
  ///         self
  ///     }
  /// }
  ///
  /// // For a type that needs reconstruction
  /// impl InPlaceTransform for String {
  ///     fn transform_in_place(self) -> Self {
  ///         self.to_uppercase()
  ///     }
  /// }
  /// ```
  fn transform_in_place(self) -> Self;
}

/// Trait for types that can be mutated in-place using a closure.
///
/// This trait enables zero-allocation in-place mutations for types that support
/// mutable references. Unlike `InPlaceTransform`, this trait works with mutable
/// references, allowing transformations without taking ownership.
///
/// # Zero-Copy Semantics
///
/// - Takes `&mut self`, allowing in-place mutation without ownership transfer
/// - Applies transformation via closure, enabling flexible mutation logic
/// - Most efficient when you have a mutable reference and want to modify in-place
///
/// # When to Use
///
/// - Use `MutateInPlace` when you have a mutable reference (`&mut T`)
/// - Prefer over `InPlaceTransform` when you don't have ownership
/// - Ideal for transformations that modify existing data structures in-place
///
/// # Example
///
/// ```rust
/// use streamweave_graph::MutateInPlace;
///
/// let mut vec = vec![1, 2, 3];
/// vec.mutate_in_place(|v| {
///     for item in v {
///         *item *= 2;
///     }
/// });
/// assert_eq!(vec, vec![2, 4, 6]);
/// ```
pub trait MutateInPlace {
  /// Mutates `self` in-place using the provided closure.
  ///
  /// This method takes a mutable reference to `self` and applies a transformation
  /// via a closure. This allows zero-allocation in-place mutations when you have
  /// a mutable reference but not ownership.
  ///
  /// # Arguments
  ///
  /// * `f` - A closure that takes `&mut Self` and applies the transformation
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::MutateInPlace;
  ///
  /// // For a type that can be mutated in-place
  /// let mut vec = vec![1, 2, 3];
  /// vec.mutate_in_place(|v| {
  ///     for item in v {
  ///         *item *= 2;
  ///     }
  /// });
  ///
  /// // For a type that needs more complex mutation
  /// let mut map = std::collections::HashMap::new();
  /// map.insert("key".to_string(), 42);
  /// map.mutate_in_place(|m| {
  ///     if let Some(value) = m.get_mut("key") {
  ///         *value *= 2;
  ///     }
  /// });
  /// ```
  fn mutate_in_place<F>(&mut self, f: F)
  where
    F: FnOnce(&mut Self);
}
