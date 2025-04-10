use effect_derive::Monad;

#[derive(Debug, Clone, PartialEq, Monad)]
struct TestBindable<T: Send + Sync + 'static> {
  value: T,
}

impl<T: Send + Sync + 'static> TestBindable<T> {
  fn pure(value: T) -> Self {
    Self { value }
  }

  fn bind<B, F>(self, f: F) -> TestBindable<B>
  where
    F: FnOnce(T) -> TestBindable<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    f(self.value)
  }
}

#[test]
fn test_bindable_derive() {
  let test = TestBindable::pure(42);
  let bound = test.bind(|x| TestBindable::pure(x.to_string()));
  assert_eq!(bound.value, "42");
}
