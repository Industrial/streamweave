use crate::functor::Functor;
use crate::monad::Monad;
use std::fmt::Debug;

/// A type that represents either success (Ok) or failure (Err).
/// Similar to Rust's built-in Result but with Effect system integration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Result<T, E> {
  Ok(T),
  Err(E),
}

impl<T, E> Result<T, E> {
  /// Creates a new Ok value.
  pub fn ok(value: T) -> Self {
    Self::Ok(value)
  }

  /// Creates a new Err value.
  pub fn err(error: E) -> Self {
    Self::Err(error)
  }

  /// Returns true if the result is Ok.
  pub fn is_ok(&self) -> bool {
    matches!(self, Self::Ok(_))
  }

  /// Returns true if the result is Err.
  pub fn is_err(&self) -> bool {
    matches!(self, Self::Err(_))
  }

  /// Unwraps the result, yielding the content of an Ok.
  pub fn unwrap(self) -> T {
    match self {
      Self::Ok(value) => value,
      Self::Err(_) => panic!("called `Result::unwrap()` on an `Err` value"),
    }
  }

  /// Unwraps the result, yielding the content of an Err.
  pub fn unwrap_err(self) -> E {
    match self {
      Self::Ok(_) => panic!("called `Result::unwrap_err()` on an `Ok` value"),
      Self::Err(error) => error,
    }
  }

  /// Returns the contained Ok value or a default.
  pub fn unwrap_or(self, default: T) -> T {
    match self {
      Self::Ok(value) => value,
      Self::Err(_) => default,
    }
  }

  /// Maps a Result<T, E> to Result<U, E> by applying a function to a contained Ok value.
  pub fn map<U, F>(self, f: F) -> Result<U, E>
  where
    F: FnOnce(T) -> U,
  {
    match self {
      Self::Ok(value) => Result::Ok(f(value)),
      Self::Err(error) => Result::Err(error),
    }
  }

  /// Maps a Result<T, E> to Result<T, F> by applying a function to a contained Err value.
  pub fn map_err<F, O>(self, f: O) -> Result<T, F>
  where
    O: FnOnce(E) -> F,
  {
    match self {
      Self::Ok(value) => Result::Ok(value),
      Self::Err(error) => Result::Err(f(error)),
    }
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Functor<T> for Result<T, E> {
  type HigherSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match self {
      Self::Ok(value) => Result::Ok(f(value)),
      Self::Err(error) => Result::Err(error),
    }
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Monad<T> for Result<T, E> {
  type HigherSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn pure(a: T) -> Self::HigherSelf<T> {
    Result::Ok(a)
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match self {
      Self::Ok(value) => f(value),
      Self::Err(error) => Result::Err(error),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_result_creation() {
    let ok = Result::<i32, &str>::ok(42);
    assert!(ok.is_ok());
    assert_eq!(ok.unwrap(), 42);

    let err = Result::<i32, &str>::err("error");
    assert!(err.is_err());
    assert_eq!(err.unwrap_err(), "error");
  }

  #[test]
  fn test_result_map() {
    let ok = Result::<i32, &str>::ok(42);
    let mapped = ok.map(|x| x * 2);
    assert_eq!(mapped.unwrap(), 84);

    let err = Result::<i32, &str>::err("error");
    let mapped = err.map(|x| x * 2);
    assert!(mapped.is_err());
  }

  #[test]
  fn test_result_map_err() {
    let ok = Result::<i32, &str>::ok(42);
    let mapped = ok.map_err(|e: &str| e.len());
    assert_eq!(mapped.unwrap(), 42);

    let err = Result::<i32, &str>::err("error");
    let mapped = err.map_err(|e| e.len());
    assert_eq!(mapped.unwrap_err(), 5);
  }

  #[test]
  fn test_result_unwrap_or() {
    let ok = Result::<i32, &str>::ok(42);
    assert_eq!(ok.unwrap_or(0), 42);

    let err = Result::<i32, &str>::err("error");
    assert_eq!(err.unwrap_or(0), 0);
  }

  #[test]
  fn test_functor_laws() {
    // Identity
    let ok = Result::<i32, &str>::ok(42);
    let ok_clone = ok.clone();
    let mapped = ok.map(|x| x);
    assert_eq!(ok_clone, mapped);

    // Composition
    let f = |x: i32| x * 2;
    let g = |x: i32| x + 1;
    let ok = Result::<i32, &str>::ok(42);
    let ok_clone = ok.clone();
    let mapped1 = ok.map(|x| g(f(x)));
    let mapped2 = ok_clone.map(f).map(g);
    assert_eq!(mapped1, mapped2);
  }

  #[test]
  fn test_monad_laws() {
    // Left identity
    let a = 42;
    let f = |x: i32| Result::<i32, &str>::ok(x * 2);
    assert_eq!(Result::<i32, &str>::pure(a).bind(f), f(a));

    // Right identity
    let m = Result::<i32, &str>::ok(42);
    let m_clone = m.clone();
    assert_eq!(m.bind(Result::pure), m_clone);

    // Associativity
    let m = Result::<i32, &str>::ok(42);
    let m_clone = m.clone();
    let f = |x: i32| Result::<i32, &str>::ok(x * 2);
    let g = |x: i32| Result::<i32, &str>::ok(x + 1);
    let f_clone = f;
    let g_clone = g;
    assert_eq!(
      m.bind(move |x| f(x).bind(g_clone)),
      m_clone.bind(f_clone).bind(g)
    );
  }
}
