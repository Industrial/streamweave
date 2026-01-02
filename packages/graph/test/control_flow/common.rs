//! Common test utilities and strategies for control flow tests

use proptest::prelude::*;
use std::time::Duration;

pub fn i32_strategy() -> impl Strategy<Value = i32> {
  -1000i32..1000i32
}

pub fn i32_vec_strategy() -> impl Strategy<Value = Vec<i32>> {
  prop::collection::vec(i32_strategy(), 0..=100)
}

#[allow(dead_code)]
pub fn duration_strategy() -> impl Strategy<Value = Duration> {
  (0u64..=1000u64).prop_map(Duration::from_millis)
}
