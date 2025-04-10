use effect_derive::{Applicative, Functor, Mappable, Monad};

#[derive(Functor, Monad, Applicative, Mappable)]
struct TestType<T: Send + Sync + 'static> {
  value: T,
}

impl<T: Send + Sync + 'static> TestType<T> {
  fn map<B, F>(self, mut f: F) -> <Self as effect_core::functor::Functor<T>>::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    TestType {
      value: f(self.value),
    }
  }

  fn pure(value: T) -> <Self as effect_core::monad::Monad<T>>::HigherSelf<T> {
    TestType { value }
  }

  fn bind<B, F>(self, mut f: F) -> <Self as effect_core::monad::Monad<T>>::HigherSelf<B>
  where
    F: FnMut(T) -> <Self as effect_core::monad::Monad<T>>::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    f(self.value)
  }

  fn ap<B, F>(
    self,
    mut f: <Self as effect_core::applicative::Applicative<T>>::HigherSelf<F>,
  ) -> <Self as effect_core::applicative::Applicative<T>>::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    TestType {
      value: (f.value)(self.value),
    }
  }
}

#[test]
fn test_all_derives() {
  // Test Functor/Mappable
  let test = TestType { value: 42 };
  let mapped = test.map(|x| x.to_string());
  assert_eq!(mapped.value, "42".to_string());

  // Test Monad
  let test = TestType::pure(42);
  let bound = test.bind(|x| TestType::pure(x.to_string()));
  assert_eq!(bound.value, "42".to_string());

  // Test Applicative
  let test = TestType::pure(42);
  let func = TestType::pure(|x: i32| x.to_string());
  let applied = test.ap(func);
  assert_eq!(applied.value, "42".to_string());
}
