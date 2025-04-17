use regex::Regex;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::pin::Pin;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

/// A trait for types that form a semigroup under some operation.
/// A semigroup is a type with an associative binary operation.
pub trait Semigroup: Sized {
  /// Combines two values using the semigroup operation.
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
macro_rules! impl_integer_semigroup {
  ($($t:ty),*) => {
    $(
      impl Semigroup for $t {
        fn combine(self, other: Self) -> Self {
          self.wrapping_add(other)
        }
      }
    )*
  };
}

impl_integer_semigroup!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

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

impl<T> Semigroup for Option<T>
where
  T: Send + Sync + Clone + 'static,
{
  fn combine(self, other: Self) -> Self {
    self.or(other)
  }
}

impl Semigroup for PathBuf {
  fn combine(self, other: Self) -> Self {
    self.join(other)
  }
}

impl Semigroup for IpAddr {
  fn combine(self, other: Self) -> Self {
    match (self, other) {
      (IpAddr::V4(a), IpAddr::V4(b)) => {
        IpAddr::V4(std::net::Ipv4Addr::from(u32::from(a) | u32::from(b)))
      }
      (IpAddr::V6(a), IpAddr::V6(b)) => {
        IpAddr::V6(std::net::Ipv6Addr::from(u128::from(a) | u128::from(b)))
      }
      _ => self,
    }
  }
}

impl Semigroup for SocketAddr {
  fn combine(self, other: Self) -> Self {
    SocketAddr::new(
      self.ip().combine(other.ip()),
      self.port().wrapping_add(other.port()),
    )
  }
}

impl Semigroup for Regex {
  fn combine(self, other: Self) -> Self {
    Regex::new(&format!("{}{}", self.as_str(), other.as_str())).unwrap()
  }
}

/// A newtype wrapper for Arc<Mutex<T>> to avoid conflicting implementations
#[derive(Clone)]
pub struct MutexArc<T>(Arc<Mutex<T>>);

/// A newtype wrapper for Arc<RwLock<T>> to avoid conflicting implementations
#[derive(Clone)]
pub struct RwLockArc<T>(Arc<RwLock<T>>);

impl<T> MutexArc<T> {
  pub fn new(value: T) -> Self {
    MutexArc(Arc::new(Mutex::new(value)))
  }

  pub fn lock(&self) -> std::sync::MutexGuard<'_, T> {
    self.0.lock().unwrap()
  }
}

impl<T> RwLockArc<T> {
  pub fn new(value: T) -> Self {
    RwLockArc(Arc::new(RwLock::new(value)))
  }

  pub fn read(&self) -> std::sync::RwLockReadGuard<'_, T> {
    self.0.read().unwrap()
  }

  pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, T> {
    self.0.write().unwrap()
  }
}

impl Semigroup for Duration {
  fn combine(self, other: Self) -> Self {
    self
      .checked_add(other)
      .unwrap_or_else(|| Duration::from_secs(u64::MAX))
  }
}

impl<T> Semigroup for MutexArc<T>
where
  T: Semigroup + Send + Sync + Clone + 'static,
{
  fn combine(self, other: Self) -> Self {
    let inner1 = self.0.lock().unwrap().clone();
    let inner2 = other.0.lock().unwrap().clone();
    MutexArc::new(inner1.combine(inner2))
  }
}

impl<T> Semigroup for RwLockArc<T>
where
  T: Semigroup + Send + Sync + Clone + 'static,
{
  fn combine(self, other: Self) -> Self {
    let inner1 = self.0.read().unwrap().clone();
    let inner2 = other.0.read().unwrap().clone();
    RwLockArc::new(inner1.combine(inner2))
  }
}

impl<T: Semigroup> Semigroup for Box<T> {
  fn combine(self, other: Self) -> Self {
    Box::new((*self).combine(*other))
  }
}

impl<T: Semigroup + Copy> Semigroup for Cell<T> {
  fn combine(self, other: Self) -> Self {
    Cell::new(self.get().combine(other.get()))
  }
}

impl<T: Semigroup> Semigroup for RefCell<T> {
  fn combine(self, other: Self) -> Self {
    RefCell::new(self.into_inner().combine(other.into_inner()))
  }
}

impl<T: Semigroup + Clone> Semigroup for Rc<T> {
  fn combine(self, other: Self) -> Self {
    Rc::new((*self).clone().combine((*other).clone()))
  }
}

impl<T: Semigroup + Clone> Semigroup for Weak<T> {
  fn combine(self, other: Self) -> Self {
    if let (Some(this), Some(other)) = (self.upgrade(), other.upgrade()) {
      Rc::downgrade(&Rc::new((*this).clone().combine((*other).clone())))
    } else {
      self
    }
  }
}

impl<T: Semigroup> Semigroup for PhantomData<T> {
  fn combine(self, _other: Self) -> Self {
    PhantomData
  }
}

impl<T: Semigroup + Clone> Semigroup for Pin<Box<T>> {
  fn combine(self, other: Self) -> Self {
    unsafe { Pin::new_unchecked(Box::new((*self).clone().combine((*other).clone()))) }
  }
}

impl<T: Semigroup + Clone> Semigroup for NonNull<T> {
  fn combine(self, other: Self) -> Self {
    unsafe {
      let this = self.as_ref();
      let other = other.as_ref();
      NonNull::new_unchecked(Box::into_raw(Box::new(this.clone().combine(other.clone()))))
    }
  }
}

impl<T: Semigroup + Copy> Semigroup for ManuallyDrop<T> {
  fn combine(self, other: Self) -> Self {
    ManuallyDrop::new({ *self }.combine(*other))
  }
}

impl<T: Semigroup + Copy> Semigroup for MaybeUninit<T> {
  fn combine(self, other: Self) -> Self {
    unsafe {
      let a = self.assume_init();
      let b = other.assume_init();
      MaybeUninit::new(a.combine(b))
    }
  }
}

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
    let self_guard = self.into_inner().unwrap();
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
    let self_guard = self.into_inner().unwrap();
    let other_guard = other.into_inner().unwrap();
    RwLock::new(self_guard.combine(other_guard))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::net::{Ipv4Addr, Ipv6Addr};
  use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem::ManuallyDrop,
    ptr::NonNull,
    rc::Rc,
  };

  // Helper function to test associativity
  fn test_associativity<T: Semigroup + Clone + PartialEq + std::fmt::Debug>(a: T, b: T, c: T) {
    let a1 = a.clone();
    let b1 = b.clone();
    let c1 = c.clone();

    let left = a.combine(b).combine(c);
    let right = a1.combine(b1.combine(c1));

    assert_eq!(left, right, "Associativity law failed");
  }

  proptest! {
    #[test]
    fn test_vec_semigroup(x: i32, y: i32, z: i32) {
      let s1 = vec![x];
      let s2 = vec![y];
      let s3 = vec![z];

      // Identity
      let s1_clone = s1.clone();
      let combined = s1_clone.combine(s2.clone());
      assert_eq!(combined, vec![x, y]);

      // Associativity
      let left = s1.clone().combine(s2.clone()).combine(s3.clone());
      let right = s1.clone().combine(s2.combine(s3));
      assert_eq!(left, right);
    }

    #[test]
    fn test_string_semigroup(s1: String, s2: String, s3: String) {
      // Test associativity
      let s1_clone = s1.clone();
      let s2_clone = s2.clone();
      let s3_clone = s3.clone();

      let combined1 = s1_clone.combine(s2_clone).combine(s3_clone);
      let combined2 = s1.combine(s2.combine(s3));
      assert_eq!(combined1, combined2);

      // Test empty strings
      let empty = String::new();
      let test_str = "test".to_string();
      let test_str_clone = test_str.clone();
      assert_eq!(empty.clone().combine(test_str.clone()), test_str_clone.clone());
      assert_eq!(test_str.combine(empty), test_str_clone);
    }

    #[test]
    fn test_integer_semigroup(x: i32, y: i32, z: i32) {
      let combined1 = x.wrapping_add(y).wrapping_add(z);
      let combined2 = x.wrapping_add(y.wrapping_add(z));
      assert_eq!(combined1, combined2);
    }

    #[test]
    fn test_float_semigroup(x: f32, y: f32, z: f32) {
      // Skip NaN, infinity values, and extremely large numbers
      prop_assume!(!x.is_nan() && !x.is_infinite() && x.abs() < 1e10);
      prop_assume!(!y.is_nan() && !y.is_infinite() && y.abs() < 1e10);
      prop_assume!(!z.is_nan() && !z.is_infinite() && z.abs() < 1e10);

      let combined1 = (x + y) + z;
      let combined2 = x + (y + z);

      // Calculate epsilon based on the magnitude of the numbers
      let max_abs = x.abs().max(y.abs()).max(z.abs());
      let epsilon = max_abs * 1e-5;

      prop_assert!((combined1 - combined2).abs() < epsilon.max(1e-5));
    }

    #[test]
    fn test_option_semigroup(x: i32, y: i32) {
      // Test Some + Some
      let some_x = Some(x);
      let some_y = Some(y);
      assert_eq!(some_x.clone().combine(some_y.clone()), Some(x));

      // Test Some + None
      assert_eq!(some_x.clone().combine(None), some_x);

      // Test None + Some
      assert_eq!(None.combine(some_y.clone()), some_y);

      // Test None + None
      assert_eq!(None::<i32>.combine(None), None);
    }

    #[test]
    fn test_pathbuf_semigroup(p1: String, p2: String, p3: String) {
      let path1 = PathBuf::from(p1.clone());
      let path2 = PathBuf::from(p2.clone());
      let path3 = PathBuf::from(p3.clone());

      // Test associativity
      let path1_clone = path1.clone();
      let path2_clone = path2.clone();
      let path3_clone = path3.clone();

      let combined1 = path1_clone.combine(path2_clone).combine(path3_clone);
      let combined2 = path1.combine(path2.combine(path3));
      assert_eq!(combined1, combined2);

      // Test empty paths
      let empty = PathBuf::new();
      let test_path = PathBuf::from("test");
      let test_path_clone = test_path.clone();
      assert_eq!(empty.clone().combine(test_path.clone()), test_path_clone.clone());
      assert_eq!(test_path.combine(empty), test_path_clone);
    }

    #[test]
    fn test_ipaddr_semigroup(
      a: u32,
      b: u32,
      c: u32,
      d: u32,
      e: u32,
      f: u32,
      g: u32,
      h: u32
    ) {
      // Test IPv4
      let ip1 = IpAddr::V4(Ipv4Addr::new(a as u8, b as u8, c as u8, d as u8));
      let ip2 = IpAddr::V4(Ipv4Addr::new(e as u8, f as u8, g as u8, h as u8));
      let combined = ip1.combine(ip2);
      assert!(matches!(combined, IpAddr::V4(_)));

      // Test IPv6
      let ip1 = IpAddr::V6(Ipv6Addr::new(
        (a % u16::MAX as u32) as u16,
        (b % u16::MAX as u32) as u16,
        (c % u16::MAX as u32) as u16,
        (d % u16::MAX as u32) as u16,
        (e % u16::MAX as u32) as u16,
        (f % u16::MAX as u32) as u16,
        (g % u16::MAX as u32) as u16,
        (h % u16::MAX as u32) as u16
      ));
      let ip2 = IpAddr::V6(Ipv6Addr::new(
        (h % u16::MAX as u32) as u16,
        (g % u16::MAX as u32) as u16,
        (f % u16::MAX as u32) as u16,
        (e % u16::MAX as u32) as u16,
        (d % u16::MAX as u32) as u16,
        (c % u16::MAX as u32) as u16,
        (b % u16::MAX as u32) as u16,
        (a % u16::MAX as u32) as u16
      ));
      let combined = ip1.combine(ip2);
      assert!(matches!(combined, IpAddr::V6(_)));

      // Test mixed versions
      let ip1 = IpAddr::V4(Ipv4Addr::new(a as u8, b as u8, c as u8, d as u8));
      let ip2 = IpAddr::V6(Ipv6Addr::new(
        (a % u16::MAX as u32) as u16,
        (b % u16::MAX as u32) as u16,
        (c % u16::MAX as u32) as u16,
        (d % u16::MAX as u32) as u16,
        (e % u16::MAX as u32) as u16,
        (f % u16::MAX as u32) as u16,
        (g % u16::MAX as u32) as u16,
        (h % u16::MAX as u32) as u16
      ));
      let combined = ip1.combine(ip2);
      assert!(matches!(combined, IpAddr::V4(_)));
    }

    #[test]
    fn test_socketaddr_semigroup(
      a: u32, b: u32, c: u32, d: u32,
      port1: u16, port2: u16
    ) {
      let ip1 = Ipv4Addr::new(a as u8, b as u8, c as u8, d as u8);
      let ip2 = Ipv4Addr::new(d as u8, c as u8, b as u8, a as u8);

      let addr1 = SocketAddr::new(IpAddr::V4(ip1), port1);
      let addr2 = SocketAddr::new(IpAddr::V4(ip2), port2);

      let combined = addr1.combine(addr2);
      assert_eq!(combined.port(), port1.wrapping_add(port2));
    }

    #[test]
    fn test_regex_semigroup(s1: String, s2: String) {
      // Skip empty strings and strings that would create invalid regex patterns
      prop_assume!(!s1.is_empty() && !s2.is_empty());
      prop_assume!(Regex::new(&regex::escape(&s1)).is_ok());
      prop_assume!(Regex::new(&regex::escape(&s2)).is_ok());

      let s1_clone = s1.clone();
      let s2_clone = s2.clone();

      let re1 = Regex::new(&regex::escape(&s1_clone)).unwrap();
      let re2 = Regex::new(&regex::escape(&s2_clone)).unwrap();

      let re1_clone = re1.clone();
      let re2_clone = re2.clone();

      let combined = re1_clone.combine(re2_clone);
      assert!(combined.is_match(&format!("{}{}", s1_clone, s2_clone)));

      // Test empty regex
      let empty = Regex::new("").unwrap();
      let test_re = Regex::new("test").unwrap();
      let test_re_clone = test_re.clone();
      assert_eq!(empty.clone().combine(test_re.clone()).as_str(), test_re_clone.as_str());
      assert_eq!(test_re.combine(empty).as_str(), test_re_clone.as_str());
    }

    #[test]
    fn test_duration_semigroup(a: u64, b: u64, c: u64) {
      let dur_a = Duration::from_millis(a);
      let dur_b = Duration::from_millis(b);
      let dur_c = Duration::from_millis(c);

      test_associativity(dur_a, dur_b, dur_c);
    }

    #[test]
    fn test_mutex_semigroup(x: i32) {
      let empty = MutexArc::new(0);
      let test = MutexArc::new(x);

      // Identity
      let test_clone = test.clone();
      let combined = test_clone.combine(empty.clone());
      assert_eq!(*combined.lock(), x);

      // Associativity
      let test_clone = test.clone();
      let combined = empty.clone().combine(test_clone);
      assert_eq!(*combined.lock(), x);
    }

    #[test]
    fn test_rwlock_semigroup(x: i32) {
      let empty = RwLockArc::new(0);
      let test = RwLockArc::new(x);

      // Identity
      let test_clone = test.clone();
      let combined = test_clone.combine(empty.clone());
      assert_eq!(*combined.read(), x);

      // Associativity
      let test_clone = test.clone();
      let combined = empty.clone().combine(test_clone);
      assert_eq!(*combined.read(), x);
    }

    #[test]
    fn test_box_semigroup(x: i32, y: i32, z: i32) {
      let a = Box::new(x);
      let b = Box::new(y);
      let c = Box::new(z);
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(combined1, combined2);
    }

    #[test]
    fn test_cell_semigroup(x: i32, y: i32, z: i32) {
      let a = Cell::new(x);
      let b = Cell::new(y);
      let c = Cell::new(z);
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(combined1.get(), combined2.get());
    }

    #[test]
    fn test_refcell_semigroup(x: i32, y: i32, z: i32) {
      let a = RefCell::new(x);
      let b = RefCell::new(y);
      let c = RefCell::new(z);
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(combined1.into_inner(), combined2.into_inner());
    }

    #[test]
    fn test_rc_semigroup(x: i32, y: i32, z: i32) {
      let a = Rc::new(x);
      let b = Rc::new(y);
      let c = Rc::new(z);
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(*combined1, *combined2);
    }

    #[test]
    fn test_weak_semigroup(x: i32, y: i32, z: i32) {
      let a = Rc::downgrade(&Rc::new(x));
      let b = Rc::downgrade(&Rc::new(y));
      let c = Rc::downgrade(&Rc::new(z));
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(combined1.upgrade(), combined2.upgrade());
    }

    #[test]
    fn test_phantomdata_semigroup(_x: i32, _y: i32, _z: i32) {
      let a: PhantomData<i32> = PhantomData;
      let b: PhantomData<i32> = PhantomData;
      let c: PhantomData<i32> = PhantomData;
      let combined1 = a.combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(std::mem::size_of_val(&combined1), std::mem::size_of_val(&combined2));
    }

    #[test]
    fn test_pin_semigroup(x: i32, y: i32, z: i32) {
      let a = Box::pin(x);
      let b = Box::pin(y);
      let c = Box::pin(z);
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(*combined1, *combined2);
    }

    #[test]
    fn test_nonnull_semigroup(x: i32, y: i32, z: i32) {
      let a = NonNull::new(Box::into_raw(Box::new(x))).unwrap();
      let b = NonNull::new(Box::into_raw(Box::new(y))).unwrap();
      let c = NonNull::new(Box::into_raw(Box::new(z))).unwrap();
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      unsafe {
        assert_eq!(*combined1.as_ref(), *combined2.as_ref());
        // Clean up
        drop(Box::from_raw(combined1.as_ptr()));
        drop(Box::from_raw(combined2.as_ptr()));
      }
    }

    #[test]
    fn test_manuallydrop_semigroup(x: i32, y: i32, z: i32) {
      let a = ManuallyDrop::new(x);
      let b = ManuallyDrop::new(y);
      let c = ManuallyDrop::new(z);
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(unsafe { *combined1 }, unsafe { *combined2 });
    }

    #[test]
    fn test_maybeuninit_semigroup(x: i32, y: i32, z: i32) {
      let a = MaybeUninit::new(x);
      let b = MaybeUninit::new(y);
      let c = MaybeUninit::new(z);
      let combined1 = a.clone().combine(b.clone()).combine(c.clone());
      let combined2 = a.combine(b.combine(c));
      assert_eq!(unsafe { combined1.assume_init() }, unsafe { combined2.assume_init() });
    }
  }
}
