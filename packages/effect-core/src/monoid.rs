use crate::semigroup::Semigroup;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

/// A trait for types that form a monoid under some operation.
/// A monoid is a type with an associative binary operation and an identity element.
pub trait Monoid: Semigroup {
  /// The identity element for the monoid operation.
  fn empty() -> Self;

  /// Fold a collection of monoid values using the monoid operation.
  fn mconcat<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = Self>,
    Self: Sized,
  {
    iter.into_iter().fold(Self::empty(), Self::combine)
  }
}

// Implement Monoid for common types

impl<T> Monoid for Vec<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    Vec::new()
  }
}

impl Monoid for String {
  fn empty() -> Self {
    String::new()
  }
}

// Implement Monoid for collections
impl<K, V> Monoid for HashMap<K, V>
where
  K: Eq + std::hash::Hash + Send + Sync + Clone + 'static,
  V: Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    HashMap::new()
  }
}

impl<T> Monoid for HashSet<T>
where
  T: Eq + std::hash::Hash + Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    HashSet::new()
  }
}

impl<K, V> Monoid for BTreeMap<K, V>
where
  K: Ord + Send + Sync + Clone + 'static,
  V: Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    BTreeMap::new()
  }
}

impl<T> Monoid for BTreeSet<T>
where
  T: Ord + Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    BTreeSet::new()
  }
}

impl<T> Monoid for LinkedList<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    LinkedList::new()
  }
}

impl<T> Monoid for BinaryHeap<T>
where
  T: Ord + Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    BinaryHeap::new()
  }
}

impl<T> Monoid for VecDeque<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    VecDeque::new()
  }
}

// Implement Monoid for reference counting
impl<T> Monoid for Arc<T>
where
  T: Monoid + Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    Arc::new(T::empty())
  }
}

// Implement Monoid for synchronization primitives
impl<T> Monoid for Mutex<T>
where
  T: Monoid + Send + 'static,
{
  fn empty() -> Self {
    Mutex::new(T::empty())
  }
}

impl<T> Monoid for RwLock<T>
where
  T: Monoid + Send + 'static,
{
  fn empty() -> Self {
    RwLock::new(T::empty())
  }
}

// Implement Monoid for time and networking
impl Monoid for Duration {
  fn empty() -> Self {
    Duration::from_secs(0)
  }
}

impl Monoid for PathBuf {
  fn empty() -> Self {
    PathBuf::new()
  }
}

impl Monoid for IpAddr {
  fn empty() -> Self {
    IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))
  }
}

impl Monoid for SocketAddr {
  fn empty() -> Self {
    SocketAddr::new(IpAddr::empty(), 0)
  }
}

// Implement Monoid for regular expressions
impl Monoid for Regex {
  fn empty() -> Self {
    Regex::new("").unwrap()
  }
}

// Implement Monoid for integer types
macro_rules! impl_integer_monoid {
  ($($t:ty),*) => {
    $(
      impl Monoid for $t {
        fn empty() -> Self {
          0
        }
      }
    )*
  };
}

impl_integer_monoid!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

// Implement Monoid for floating-point types
macro_rules! impl_float_monoid {
  ($($t:ty),*) => {
    $(
      impl Monoid for $t {
        fn empty() -> Self {
          0.0
        }
      }
    )*
  };
}

impl_float_monoid!(f32, f64);

impl<T> Monoid for Option<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn empty() -> Self {
    None
  }
}

// Implement Semigroup for collections
impl<K, V> Semigroup for HashMap<K, V>
where
  K: Eq + std::hash::Hash + Send + Sync + Clone + 'static,
  V: Send + Sync + Clone + 'static,
{
  fn combine(mut self, other: Self) -> Self {
    self.extend(other);
    self
  }
}

impl<T> Semigroup for HashSet<T>
where
  T: Eq + std::hash::Hash + Send + Sync + Clone + 'static,
{
  fn combine(mut self, other: Self) -> Self {
    self.extend(other);
    self
  }
}

impl<K, V> Semigroup for BTreeMap<K, V>
where
  K: Ord + Send + Sync + Clone + 'static,
  V: Send + Sync + Clone + 'static,
{
  fn combine(mut self, other: Self) -> Self {
    self.extend(other);
    self
  }
}

impl<T> Semigroup for BTreeSet<T>
where
  T: Ord + Send + Sync + Clone + 'static,
{
  fn combine(mut self, other: Self) -> Self {
    self.extend(other);
    self
  }
}

impl<T> Semigroup for LinkedList<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn combine(mut self, mut other: Self) -> Self {
    self.append(&mut other);
    self
  }
}

impl<T> Semigroup for BinaryHeap<T>
where
  T: Ord + Send + Sync + Clone + 'static,
{
  fn combine(mut self, mut other: Self) -> Self {
    self.append(&mut other);
    self
  }
}

impl<T> Semigroup for VecDeque<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn combine(mut self, mut other: Self) -> Self {
    self.append(&mut other);
    self
  }
}

// Implement Semigroup for Arc
impl<T> Semigroup for Arc<T>
where
  T: Semigroup + Send + Sync + Clone + 'static,
{
  fn combine(self, other: Self) -> Self {
    Arc::new((*self).clone().combine((*other).clone()))
  }
}

// Implement Semigroup for Mutex
impl<T> Semigroup for Mutex<T>
where
  T: Semigroup + Send + 'static,
{
  fn combine(self, other: Self) -> Self {
    let mut self_guard = self.into_inner().unwrap();
    let other_guard = other.into_inner().unwrap();
    Mutex::new(self_guard.combine(other_guard))
  }
}

// Implement Semigroup for RwLock
impl<T> Semigroup for RwLock<T>
where
  T: Semigroup + Send + 'static,
{
  fn combine(self, other: Self) -> Self {
    let mut self_guard = self.into_inner().unwrap();
    let other_guard = other.into_inner().unwrap();
    RwLock::new(self_guard.combine(other_guard))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use regex::Regex;
  use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
  use std::net::{IpAddr, SocketAddr};
  use std::path::PathBuf;
  use std::sync::{Arc, Mutex, RwLock};
  use std::time::Duration;

  proptest! {
    #[test]
    fn test_monoid_laws_vec(x: Vec<i32>, y: Vec<i32>, z: Vec<i32>) {
      // Identity
      assert_eq!(x.clone().combine(Vec::empty()), x);
      assert_eq!(Vec::empty().combine(x.clone()), x);

      // Associativity
      assert_eq!(
        x.clone().combine(y.clone()).combine(z.clone()),
        x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_hashmap(
      x: HashMap<i32, i32>,
      y: HashMap<i32, i32>,
      z: HashMap<i32, i32>
    ) {
      // Identity
      assert_eq!(x.clone().combine(HashMap::empty()), x);
      assert_eq!(HashMap::empty().combine(x.clone()), x);

      // Associativity
      assert_eq!(
        x.clone().combine(y.clone()).combine(z.clone()),
        x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_hashset(
      x: HashSet<i32>,
      y: HashSet<i32>,
      z: HashSet<i32>
    ) {
      // Identity
      assert_eq!(x.clone().combine(HashSet::empty()), x);
      assert_eq!(HashSet::empty().combine(x.clone()), x);

      // Associativity
      assert_eq!(
        x.clone().combine(y.clone()).combine(z.clone()),
        x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_btreemap(
      x: BTreeMap<i32, i32>,
      y: BTreeMap<i32, i32>,
      z: BTreeMap<i32, i32>
    ) {
      // Identity
      assert_eq!(x.clone().combine(BTreeMap::empty()), x);
      assert_eq!(BTreeMap::empty().combine(x.clone()), x);

      // Associativity
      assert_eq!(
        x.clone().combine(y.clone()).combine(z.clone()),
        x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_btreeset(
      x: BTreeSet<i32>,
      y: BTreeSet<i32>,
      z: BTreeSet<i32>
    ) {
      // Identity
      assert_eq!(x.clone().combine(BTreeSet::empty()), x);
      assert_eq!(BTreeSet::empty().combine(x.clone()), x);

      // Associativity
      assert_eq!(
        x.clone().combine(y.clone()).combine(z.clone()),
        x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_linkedlist(
      x: LinkedList<i32>,
      y: LinkedList<i32>,
      z: LinkedList<i32>
    ) {
      // Identity
      assert_eq!(x.clone().combine(LinkedList::empty()), x);
      assert_eq!(LinkedList::empty().combine(x.clone()), x);

      // Associativity
      assert_eq!(
        x.clone().combine(y.clone()).combine(z.clone()),
        x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_binaryheap(
      x: Vec<i32>,
      y: Vec<i32>,
      z: Vec<i32>
    ) {
      let x = BinaryHeap::from(x);
      let y = BinaryHeap::from(y);
      let z = BinaryHeap::from(z);

      // Identity
      let empty = BinaryHeap::empty();
      let x_clone = x.clone();
      let combined = x_clone.clone().combine(empty.clone());
      assert_eq!(combined.into_sorted_vec(), x.clone().into_sorted_vec());

      let combined = empty.combine(x_clone);
      assert_eq!(combined.into_sorted_vec(), x.clone().into_sorted_vec());

      // Associativity
      let left = x.clone().combine(y.clone()).combine(z.clone());
      let right = x.combine(y.combine(z));
      assert_eq!(left.into_sorted_vec(), right.into_sorted_vec());
    }

    #[test]
    fn test_monoid_laws_vecdeque(
      x: VecDeque<i32>,
      y: VecDeque<i32>,
      z: VecDeque<i32>
    ) {
      // Identity
      assert_eq!(x.clone().combine(VecDeque::empty()), x);
      assert_eq!(VecDeque::empty().combine(x.clone()), x);

      // Associativity
      assert_eq!(
        x.clone().combine(y.clone()).combine(z.clone()),
        x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_arc(x: i32, y: i32, z: i32) {
      let x = Arc::new(x);
      let y = Arc::new(y);
      let z = Arc::new(z);

      // Identity
      assert_eq!(*x.clone().combine(Arc::new(0)), *x);
      assert_eq!(*Arc::new(0).combine(x.clone()), *x);

      // Associativity
      assert_eq!(
        *x.clone().combine(y.clone()).combine(z.clone()),
        *x.combine(y.combine(z))
      );
    }

    #[test]
    fn test_monoid_laws_mutex(x: i32, y: i32, z: i32) {
      let x = Mutex::new(x);
      let y = Mutex::new(y);
      let z = Mutex::new(z);

      // Identity
      let x_guard = x.lock().unwrap();
      let empty_mutex = Mutex::new(0);
      let empty_guard = empty_mutex.lock().unwrap();
      let combined = x_guard.combine(*empty_guard);
      assert_eq!(combined, *x_guard);

      let empty_mutex = Mutex::new(0);
      let empty_guard = empty_mutex.lock().unwrap();
      let x_guard = x.lock().unwrap();
      let combined = empty_guard.combine(*x_guard);
      assert_eq!(combined, *x_guard);

      // Associativity
      let x_guard = x.lock().unwrap();
      let y_guard = y.lock().unwrap();
      let z_guard = z.lock().unwrap();
      let left = x_guard.combine(*y_guard).combine(*z_guard);
      let right = x_guard.combine(y_guard.combine(*z_guard));
      assert_eq!(left, right);
    }

    #[test]
    fn test_monoid_laws_rwlock(x: i32, y: i32, z: i32) {
      let x = RwLock::new(x);
      let y = RwLock::new(y);
      let z = RwLock::new(z);

      // Identity
      let x_guard = x.write().unwrap();
      let empty_rwlock = RwLock::new(0);
      let empty_guard = empty_rwlock.write().unwrap();
      let combined = x_guard.combine(*empty_guard);
      assert_eq!(combined, *x_guard);

      let empty_rwlock = RwLock::new(0);
      let empty_guard = empty_rwlock.write().unwrap();
      let x_guard = x.write().unwrap();
      let combined = empty_guard.combine(*x_guard);
      assert_eq!(combined, *x_guard);

      // Associativity
      let x_guard = x.write().unwrap();
      let y_guard = y.write().unwrap();
      let z_guard = z.write().unwrap();
      let left = x_guard.combine(*y_guard).combine(*z_guard);
      let right = x_guard.combine(y_guard.combine(*z_guard));
      assert_eq!(left, right);
    }

    #[test]
    fn test_monoid_laws_duration(x: Duration, y: Duration, z: Duration) {
      // Identity
      assert_eq!(x.combine(Duration::from_secs(0)), x);
      assert_eq!(Duration::from_secs(0).combine(x), x);

      // Associativity
      assert_eq!(
        x.combine(y).combine(z),
        x.combine(y.combine(z))
      );
    }
  }
}
