use effect_derive::Functor;

#[derive(Functor)]
struct TestMappable<T> {
  value: T,
}

impl<T> TestMappable<T> {
  fn map<U, F>(self, mut f: F) -> TestMappable<U>
  where
    F: FnMut(T) -> U,
  {
    TestMappable {
      value: f(self.value),
    }
  }
}

#[test]
fn test_mappable() {
  let test = TestMappable { value: 42 };
  let mapped = test.map(|x| x.to_string());
  assert_eq!(mapped.value, "42".to_string());
}
