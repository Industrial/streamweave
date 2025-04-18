use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Foldable<B> for (A, B) {
  fn fold<Acc, F>(self, init: Acc, mut f: F) -> Acc
  where
    Acc: CloneableThreadSafe,
    F: for<'a> FnMut(Acc, &'a B) -> Acc + CloneableThreadSafe,
  {
    // For a tuple, fold over the second element
    f(init, &self.1)
  }

  fn fold_right<Acc, F>(self, init: Acc, mut f: F) -> Acc
  where
    Acc: CloneableThreadSafe,
    F: for<'a> FnMut(&'a B, Acc) -> Acc + CloneableThreadSafe,
  {
    // For a tuple, fold_right over the second element
    f(&self.1, init)
  }

  fn reduce<F>(self, _: F) -> Option<B>
  where
    F: for<'a, 'b> FnMut(&'a B, &'b B) -> B + CloneableThreadSafe,
  {
    // For a single value, reduce just returns the value
    Some(self.1)
  }

  fn reduce_right<F>(self, _: F) -> Option<B>
  where
    F: for<'a, 'b> FnMut(&'a B, &'b B) -> B + CloneableThreadSafe,
  {
    // For a single value, reduce_right just returns the value
    Some(self.1)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_fold() {
    let pair = (10, 5);
    let result = Foldable::fold(pair, 0, |acc, v| acc + v);
    assert_eq!(result, 5); // Only the second element (5) is used

    let pair = (10, "hello");
    let result = Foldable::fold(pair, 0, |acc, s| acc + s.len());
    assert_eq!(result, 5); // "hello".len() == 5
  }

  #[test]
  fn test_fold_right() {
    let pair = (10, "hello");
    let result = Foldable::fold_right(pair, "".to_string(), |s, acc| format!("{}{}", acc, s));
    assert_eq!(result, "hello");

    let pair = (10, 5);
    let result = Foldable::fold_right(pair, 1, |v, acc| acc * v);
    assert_eq!(result, 5); // 1 * 5 = 5
  }

  #[test]
  fn test_reduce() {
    let pair = (10, "hello");
    let result = Foldable::reduce(pair, |_, _| unreachable!()).unwrap();
    assert_eq!(result, "hello");

    let pair = (10, 5);
    let result = Foldable::reduce(pair, |_, _| unreachable!()).unwrap();
    assert_eq!(result, 5);
  }

  #[test]
  fn test_reduce_right() {
    let pair = (10, "hello");
    let result = Foldable::reduce_right(pair, |_, _| unreachable!()).unwrap();
    assert_eq!(result, "hello");

    let pair = (10, 5);
    let result = Foldable::reduce_right(pair, |_, _| unreachable!()).unwrap();
    assert_eq!(result, 5);
  }
}
