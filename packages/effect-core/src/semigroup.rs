/// A trait for types that form a semigroup under some operation.
/// A semigroup is a type with an associative binary operation.
pub trait Semigroup: Sized {
  /// Combine two values using the semigroup operation.
  fn combine(self, other: Self) -> Self;
}

// Implement Semigroup for common types

impl<T: Clone> Semigroup for Vec<T> {
  fn combine(mut self, mut other: Self) -> Self {
    self.append(&mut other);
    self
  }
}

impl Semigroup for String {
  fn combine(mut self, other: Self) -> Self {
    self.push_str(&other);
    self
  }
}

// Implement Semigroup for integer types
macro_rules! impl_semigroup_for_numbers {
    ($($t:ty),*) => {
        $(
            impl Semigroup for $t {
                fn combine(self, other: Self) -> Self {
                    match self.checked_add(other) {
                        Some(result) => result,
                        None => {
                            if other > 0 {
                                Self::MAX
                            } else {
                                Self::MIN
                            }
                        }
                    }
                }
            }
        )*
    }
}

impl_semigroup_for_numbers!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

// Implement Semigroup for floating-point types
macro_rules! impl_float_semigroup {
    ($($t:ty),*) => {
        $(
            impl Semigroup for $t {
                fn combine(self, other: Self) -> Self {
                    self + other
                }
            }
        )*
    };
}

impl_float_semigroup!(f32, f64);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_semigroup() {
    let v1 = vec![1, 2, 3];
    let v2 = vec![4, 5, 6];
    let combined = v1.combine(v2);
    assert_eq!(combined, vec![1, 2, 3, 4, 5, 6]);
  }

  #[test]
  fn test_string_semigroup() {
    let s1 = "hello".to_string();
    let s2 = " world".to_string();
    let combined = s1.combine(s2);
    assert_eq!(combined, "hello world");
  }

  #[test]
  fn test_integer_semigroup() {
    let a: i32 = i32::MAX - 1;
    let b: i32 = 2;
    assert_eq!(a.combine(b), i32::MAX); // Should saturate at MAX instead of overflowing

    let c: i32 = i32::MIN + 1;
    let d: i32 = -2;
    assert_eq!(c.combine(d), i32::MIN); // Should saturate at MIN instead of overflowing
  }

  #[test]
  fn test_float_semigroup() {
    let sum = 1.0_f32.combine(2.0);
    assert_eq!(sum, 3.0);
  }
}
