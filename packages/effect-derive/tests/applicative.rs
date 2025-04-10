use effect_derive::Applicative;

#[derive(Debug, PartialEq, Clone, Applicative)]
pub struct TestApplicative<T: Send + Sync + 'static> {
  pub value: T,
}

impl<T: Send + Sync + 'static> TestApplicative<T> {
  fn new(value: T) -> Self {
    Self { value }
  }

  fn pure(value: T) -> Self {
    Self { value }
  }

  fn ap<B, F>(self, mut f: TestApplicative<F>) -> TestApplicative<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    TestApplicative {
      value: (f.value)(self.value),
    }
  }
}

#[test]
fn test_applicative() {
  let x = TestApplicative::new(42);
  let f = TestApplicative::new(|x: i32| x.to_string());
  let result = x.ap(f);
  assert_eq!(result.value, "42");
}

// Identity: pure id <*> v = v
#[test]
fn test_applicative_identity() {
  let v = TestApplicative::new(42);
  let id = TestApplicative::new(|x: i32| x);
  let result = v.clone().ap(id);
  assert_eq!(result, v);
}

// Homomorphism: pure f <*> pure x = pure (f x)
#[test]
fn test_applicative_homomorphism() {
  let f = |x: i32| x.to_string();
  let x = 42;
  let left = TestApplicative::pure(x).ap(TestApplicative::pure(f));
  let right = TestApplicative::pure(f(x));
  assert_eq!(left, right);
}

// Interchange: u <*> pure y = pure ($ y) <*> u
#[test]
fn test_applicative_interchange() {
  let u = TestApplicative::new(|x: i32| x.to_string());
  let y = 42;
  let left = TestApplicative::pure(y).ap(u.clone());
  let right = TestApplicative::pure(y).ap(u);
  assert_eq!(left, right);
}

// Composition: u <*> (v <*> w)
#[test]
fn test_applicative_composition() {
  let w = TestApplicative::new(42);
  let v = TestApplicative::new(|x: i32| x * 2);
  let u = TestApplicative::new(|x: i32| x.to_string());

  let result1 = w.clone().ap(v.clone()).ap(u.clone());
  let result2 = w.ap(v).ap(u);

  assert_eq!(result1, result2);
}
