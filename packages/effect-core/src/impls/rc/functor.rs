use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use proptest::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

impl<T: CloneableThreadSafe> Functor<T> for Rc<T> {
  type HigherSelf<U: CloneableThreadSafe> = RcWrapper<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    RcWrapper::new(f(&self))
  }
}

#[derive(Debug, Clone)]
pub struct RcWrapper<T: CloneableThreadSafe>(Arc<T>);

impl<T: CloneableThreadSafe> RcWrapper<T> {
  pub fn new(value: T) -> Self {
    RcWrapper(Arc::new(value))
  }

  pub fn get(&self) -> &T {
    &self.0
  }

  pub fn to_rc(&self) -> Rc<T>
  where
    T: Clone,
  {
    Rc::new((*self.0).clone())
  }

  pub fn to_arc(&self) -> Arc<T> {
    self.0.clone()
  }
}

impl<T: CloneableThreadSafe> std::ops::Deref for RcWrapper<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T: CloneableThreadSafe> CloneableThreadSafe for RcWrapper<T> {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::functor::Functor;

  #[test]
  fn test_functor_identity() {
    let rc = Rc::new(42);
    let mapped = Functor::map(rc.clone(), |x| *x);
    assert_eq!(*mapped, 42);
  }

  #[test]
  fn test_composition() {
    let rc = Rc::new(42);

    let f = |x: &i32| x + 1;
    let g = |x: &i32| x * 2;

    let composed_fn = move |x: &i32| g(&f(x));
    let direct_composed = Functor::map(rc.clone(), composed_fn);

    let step1 = Functor::map(rc, f);
    let step2 = g(step1.get());

    assert_eq!(*direct_composed, step2);
    assert_eq!(step2, 86); // (42 + 1) * 2 = 86
  }

  #[test]
  fn test_functor_laws() {
    let rc = Rc::new("hello".to_string());
    let mapped_id = Functor::map(rc.clone(), |x| x.clone());
    assert_eq!(*mapped_id, *rc);

    let rc_num = Rc::new(5);
    let f = |x: &i32| x * 2;
    let g = |x: &i32| x + 3;

    let composed_fn = move |x: &i32| g(&f(x));
    let composed_result = Functor::map(rc_num.clone(), composed_fn);

    let first_step = Functor::map(rc_num.clone(), f);
    let manual_result = g(first_step.get());

    assert_eq!(*composed_result, manual_result);
    assert_eq!(manual_result, 13); // (5 * 2) + 3 = 13
  }

  #[test]
  fn test_functor_with_complex_types() {
    let rc = Rc::new(vec![1, 2, 3]);
    let mapped = Functor::map(rc, |v| v.iter().sum::<i32>());
    assert_eq!(*mapped, 6);
  }

  #[test]
  fn test_wrapper_functions() {
    let rc = Rc::new(42);
    let mapped = Functor::map(rc, |x| x * 2);

    assert_eq!(*mapped, 84);

    assert_eq!(*mapped.get(), 84);

    let arc = mapped.to_arc();
    assert_eq!(*arc, 84);

    let new_rc = mapped.to_rc();
    assert_eq!(*new_rc, 84);
  }

  // Additional tests for comprehensive coverage

  // Property-based test for identity law
  proptest! {
    #[test]
    fn test_functor_identity_prop(x in -1000..1000i32) {
      let rc = Rc::new(x);
      let mapped = Functor::map(rc.clone(), |n| *n);

      prop_assert_eq!(*mapped, x);
    }
  }

  // Property-based test for composition law
  proptest! {
    #[test]
    fn test_functor_composition_prop(
      x in -1000..1000i32,
      a in -10..10i32,
      b in -10..10i32
    ) {
      let rc = Rc::new(x);

      let f = move |n: &i32| n + a;
      let g = move |n: &i32| n * b;

      // First approach: compose then map
      let composed = move |n: &i32| g(&f(n));
      let result1 = Functor::map(rc.clone(), composed);

      // Second approach: map twice
      let step1 = Functor::map(rc, f);
      let result2 = g(step1.get());

      prop_assert_eq!(*result1, result2);
    }
  }

  // Test with various reference types
  #[test]
  fn test_functor_with_refs() {
    // &str -> String
    let rc = Rc::new("hello");
    let mapped = Functor::map(rc, |s| s.to_uppercase());
    assert_eq!(*mapped, "HELLO");

    // String reference
    let rc = Rc::new("world".to_string());
    let mapped = Functor::map(rc, |s| s.len());
    assert_eq!(*mapped, 5);
  }

  // Test with Option
  #[test]
  fn test_functor_with_option() {
    // Map Some
    let rc = Rc::new(Some(42));
    let mapped = Functor::map(rc, |opt| opt.map(|x| x * 2));
    assert_eq!(*mapped, Some(84));

    // Map None
    let rc = Rc::new(None::<i32>);
    let mapped = Functor::map(rc, |opt| opt.map(|x| x * 2));
    assert_eq!(*mapped, None);
  }

  // Test with Result
  #[test]
  fn test_functor_with_result() {
    // Map Ok
    let rc = Rc::new(Ok::<_, &str>(42));
    let mapped = Functor::map(rc, |res| res.map(|x| x * 2));
    assert_eq!(*mapped, Ok(84));

    // Map Err
    let rc = Rc::new(Err::<i32, _>("error"));
    let mapped = Functor::map(rc, |res| res.map(|x| x * 2));
    assert_eq!(*mapped, Err("error"));
  }

  // Test with nested containers
  #[test]
  fn test_functor_with_nested_containers() {
    // Nested Vec
    let rc = Rc::new(vec![vec![1, 2], vec![3, 4]]);
    let mapped = Functor::map(rc, |vecs| {
      vecs
        .iter()
        .map(|v| v.iter().sum::<i32>())
        .collect::<Vec<_>>()
    });
    assert_eq!(*mapped, vec![3, 7]);

    // Nested Option
    let rc = Rc::new(Some(Some(42)));
    let mapped = Functor::map(rc, |opt| opt.map(|inner| inner.map(|x| x * 2)));
    assert_eq!(*mapped, Some(Some(84)));
  }

  // Test thread safety (compile-time check)
  #[test]
  fn test_functor_thread_safety() {
    let rc = Rc::new(42);
    let mapped = Functor::map(rc, |x| x * 2);

    // This effectively tests that RcWrapper implements Send + Sync
    let handle = std::thread::spawn(move || {
      assert_eq!(*mapped, 84);
    });

    handle.join().unwrap();
  }

  // Test with mutable operations inside mapping function
  #[test]
  fn test_functor_with_mutable_operations() {
    let rc = Rc::new(vec![1, 2, 3]);
    let mut sum = 0;

    let mapped = Functor::map(rc, |vec| {
      for &x in vec {
        sum += x;
      }
      sum
    });

    assert_eq!(*mapped, 6);
    assert_eq!(sum, 6);
  }

  // Test RcWrapper helper methods specifically
  #[test]
  fn test_rc_wrapper_methods() {
    let value = 42;
    let wrapper = RcWrapper::new(value);

    // Test get
    assert_eq!(*wrapper.get(), 42);

    // Test to_arc
    let arc = wrapper.to_arc();
    assert_eq!(*arc, 42);

    // Test to_rc
    let rc = wrapper.to_rc();
    assert_eq!(*rc, 42);

    // Test Deref
    assert_eq!(*wrapper, 42);
  }
}
