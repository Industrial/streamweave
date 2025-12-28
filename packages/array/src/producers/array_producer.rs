use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that emits items from a fixed-size array.
///
/// This producer iterates over the elements of an internal array and
/// emits them as a stream. The array size is determined at compile time
/// via the const generic parameter `N`.
pub struct ArrayProducer<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> {
  /// The array containing the data to be produced.
  pub array: [T; N],
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> ArrayProducer<T, N> {
  /// Creates a new `ArrayProducer` with the given array.
  ///
  /// # Arguments
  ///
  /// * `array` - The array whose elements will be produced.
  pub fn new(array: [T; N]) -> Self {
    Self {
      array,
      config: streamweave::ProducerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Creates an `ArrayProducer` from a slice, if the slice length matches `N`.
  ///
  /// # Arguments
  ///
  /// * `slice` - The slice to convert to an array.
  ///
  /// # Returns
  ///
  /// Returns `Some(ArrayProducer)` if the slice length equals `N`, otherwise `None`.
  ///
  /// # Safety
  ///
  /// This function uses unsafe code to initialize the array from the slice.
  /// The caller must ensure that `T: Copy` is satisfied.
  pub fn from_slice(slice: &[T]) -> Option<Self>
  where
    T: Copy,
  {
    if slice.len() != N {
      return None;
    }
    let mut arr = std::mem::MaybeUninit::<[T; N]>::uninit();
    let ptr = arr.as_mut_ptr() as *mut T;
    unsafe {
      for (i, &item) in slice.iter().enumerate() {
        ptr.add(i).write(item);
      }
      Some(Self::new(arr.assume_init()))
    }
  }

  /// Returns the length of the array (always `N`).
  pub fn len(&self) -> usize {
    N
  }

  /// Returns `true` if the array is empty (i.e., `N == 0`).
  pub fn is_empty(&self) -> bool {
    N == 0
  }
}
