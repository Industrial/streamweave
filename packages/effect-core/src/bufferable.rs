use std::option::Option;
use std::vec::Vec;

/// A trait for types that can buffer their elements with different strategies.
pub trait Bufferable<T> {
  /// Buffers elements with a fixed size, dropping oldest elements when full.
  fn buffer_drop_oldest(self, size: usize) -> Self;

  /// Buffers elements with a fixed size, dropping newest elements when full.
  fn buffer_drop_newest(self, size: usize) -> Self;

  /// Buffers elements with a fixed size, returning an error when full.
  fn buffer_fixed(self, size: usize) -> Result<Self, String>
  where
    Self: Sized;
}

impl<T: Clone> Bufferable<T> for Vec<T> {
  fn buffer_drop_oldest(self, size: usize) -> Vec<T> {
    let len = self.len();
    if len <= size {
      return self;
    }
    self.into_iter().skip(len - size).collect()
  }

  fn buffer_drop_newest(self, size: usize) -> Vec<T> {
    if self.len() <= size {
      return self;
    }
    self.into_iter().take(size).collect()
  }

  fn buffer_fixed(self, size: usize) -> Result<Vec<T>, String> {
    if self.len() > size {
      Err(format!("Buffer overflow: size {} exceeded", size))
    } else {
      Ok(self)
    }
  }
}

impl<T: Clone> Bufferable<T> for Option<T> {
  fn buffer_drop_oldest(self, _size: usize) -> Option<T> {
    self
  }

  fn buffer_drop_newest(self, _size: usize) -> Option<T> {
    self
  }

  fn buffer_fixed(self, size: usize) -> Result<Option<T>, String> {
    if size == 0 {
      Err("Buffer size must be at least 1".to_string())
    } else {
      Ok(self)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_buffer_drop_oldest() {
    let v = vec![1, 2, 3, 4, 5];
    assert_eq!(v.clone().buffer_drop_oldest(3), vec![3, 4, 5]);
    assert_eq!(v.clone().buffer_drop_oldest(5), v);
    assert_eq!(v.clone().buffer_drop_oldest(6), v);
  }

  #[test]
  fn test_vec_buffer_drop_newest() {
    let v = vec![1, 2, 3, 4, 5];
    assert_eq!(v.clone().buffer_drop_newest(3), vec![1, 2, 3]);
    assert_eq!(v.clone().buffer_drop_newest(5), v);
    assert_eq!(v.clone().buffer_drop_newest(6), v);
  }

  #[test]
  fn test_vec_buffer_fixed() {
    let v = vec![1, 2, 3, 4, 5];
    assert!(v.clone().buffer_fixed(3).is_err());
    assert_eq!(v.clone().buffer_fixed(5).unwrap(), v);
    assert_eq!(v.clone().buffer_fixed(6).unwrap(), v);
  }

  #[test]
  fn test_option_buffer_drop_oldest() {
    let o: Option<i32> = Some(42);
    assert_eq!(o.buffer_drop_oldest(1), Some(42));
    assert_eq!(o.buffer_drop_oldest(2), Some(42));

    let none: Option<i32> = None;
    assert_eq!(none.buffer_drop_oldest(1), None);
  }

  #[test]
  fn test_option_buffer_drop_newest() {
    let o: Option<i32> = Some(42);
    assert_eq!(o.buffer_drop_newest(1), Some(42));
    assert_eq!(o.buffer_drop_newest(2), Some(42));

    let none: Option<i32> = None;
    assert_eq!(none.buffer_drop_newest(1), None);
  }

  #[test]
  fn test_option_buffer_fixed() {
    let o: Option<i32> = Some(42);
    assert_eq!(o.buffer_fixed(1).unwrap(), Some(42));
    assert_eq!(o.buffer_fixed(2).unwrap(), Some(42));

    let none: Option<i32> = None;
    assert_eq!(none.buffer_fixed(1).unwrap(), None);
    assert!(none.buffer_fixed(0).is_err());
  }
}
