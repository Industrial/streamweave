use effect_derive::Functor;

#[derive(Functor)]
struct TestFunctor<T> {
  value: T,
}

impl<T> TestFunctor<T> {
  fn map<U, F>(self, mut f: F) -> TestFunctor<U>
  where
    F: FnMut(T) -> U,
  {
    TestFunctor {
      value: f(self.value),
    }
  }
}

#[test]
fn test_functor() {
  let test = TestFunctor { value: 42 };
  let mapped = test.map(|x| x.to_string());
  assert_eq!(mapped.value, "42".to_string());
}
