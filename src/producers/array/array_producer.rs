use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

pub struct ArrayProducer<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> {
  pub array: [T; N],
  pub config: ProducerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> ArrayProducer<T, N> {
  pub fn new(array: [T; N]) -> Self {
    Self {
      array,
      config: crate::producer::ProducerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

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

  pub fn len(&self) -> usize {
    N
  }

  pub fn is_empty(&self) -> bool {
    N == 0
  }
}
