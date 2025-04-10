use effect_derive::Monad;

#[derive(Monad)]
struct TestMonad<T> {
  value: T,
}

impl<T> TestMonad<T> {
  fn unit(value: T) -> Self {
    TestMonad { value }
  }

  fn bind<B, F>(self, f: F) -> TestMonad<B>
  where
    F: FnOnce(T) -> TestMonad<B>,
  {
    f(self.value)
  }
}

#[test]
fn test_monad_derive() {
  let test = TestMonad::unit(42);
  let bound = test.bind(|x| TestMonad::unit(x.to_string()));
  assert_eq!(bound.value, "42");
}
