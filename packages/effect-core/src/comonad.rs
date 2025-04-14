use super::functor::Functor;

pub trait Comonad<T>: Functor<T> {
  fn extract<A>(wa: Self::HigherSelf<A>) -> A
  where
    A: Send + Sync + 'static,
    Self::HigherSelf<A>: Send + Sync;

  fn duplicate<A>(wa: Self::HigherSelf<A>) -> Self::HigherSelf<Self::HigherSelf<A>>
  where
    A: Send + Sync + 'static + Clone,
    Self::HigherSelf<A>: Send + Sync + Clone;

  fn extend<A, B, F>(wa: Self::HigherSelf<A>, f: F) -> Self::HigherSelf<B>
  where
    A: Send + Sync + 'static + Clone,
    B: Send + Sync + 'static,
    F: Fn(Self::HigherSelf<A>) -> B + Send + Sync + 'static,
    Self::HigherSelf<A>: Send + Sync + Clone;
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::sync::Arc;

  // Example implementation for NonEmptyVec
  #[derive(Clone, Debug, PartialEq)]
  struct NonEmptyVec<A> {
    head: A,
    tail: Vec<A>,
  }

  impl<A> NonEmptyVec<A> {
    fn new(head: A, tail: Vec<A>) -> Self {
      NonEmptyVec { head, tail }
    }

    fn singleton(head: A) -> Self {
      NonEmptyVec { head, tail: vec![] }
    }

    fn to_vec(&self) -> Vec<A>
    where
      A: Clone,
    {
      let mut result = vec![self.head.clone()];
      result.extend(self.tail.clone());
      result
    }
  }

  impl<T> Functor<T> for NonEmptyVec<T> {
    type HigherSelf<U: Send + Sync + 'static> = NonEmptyVec<U>;

    fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
    where
      F: FnMut(T) -> B + Send + Sync + 'static,
      B: Send + Sync + 'static,
    {
      let head = f(self.head);
      let tail = self.tail.into_iter().map(f).collect();
      NonEmptyVec { head, tail }
    }
  }

  impl<T> Comonad<T> for NonEmptyVec<T> {
    fn extract<A>(wa: Self::HigherSelf<A>) -> A
    where
      A: Send + Sync + 'static,
      Self::HigherSelf<A>: Send + Sync,
    {
      wa.head
    }

    fn duplicate<A>(wa: Self::HigherSelf<A>) -> Self::HigherSelf<Self::HigherSelf<A>>
    where
      A: Send + Sync + 'static + Clone,
      Self::HigherSelf<A>: Send + Sync + Clone,
    {
      let head = wa.clone();
      let mut tail = Vec::new();
      let mut current = wa.tail.clone();

      while !current.is_empty() {
        tail.push(NonEmptyVec::new(current[0].clone(), current[1..].to_vec()));
        current = current[1..].to_vec();
      }

      NonEmptyVec { head, tail }
    }

    fn extend<A, B, F>(wa: Self::HigherSelf<A>, f: F) -> Self::HigherSelf<B>
    where
      A: Send + Sync + 'static + Clone,
      B: Send + Sync + 'static,
      F: Fn(Self::HigherSelf<A>) -> B + Send + Sync + 'static,
      Self::HigherSelf<A>: Send + Sync + Clone,
    {
      let duplicated = Self::duplicate(wa);
      duplicated.map(f)
    }
  }

  proptest! {
      #[test]
      fn test_comonad_extract(x: i32, xs in prop::collection::vec(any::<i32>(), 0..10)) {
          let nev = NonEmptyVec::new(x, xs.clone());
          assert_eq!(<NonEmptyVec<i32> as Comonad<i32>>::extract(nev), x);
      }

      #[test]
      fn test_comonad_duplicate(x: i32, xs in prop::collection::vec(any::<i32>(), 0..5)) {
          let nev = NonEmptyVec::new(x, xs.clone());
          let duplicated = <NonEmptyVec<i32> as Comonad<i32>>::duplicate(nev.clone());

          // Test that extract of duplicated equals original
          assert_eq!(
              <NonEmptyVec<i32> as Comonad<i32>>::extract(duplicated),
              nev
          );
      }

      #[test]
      fn test_comonad_extend(x: i32, xs in prop::collection::vec(any::<i32>(), 0..5)) {
          let nev = NonEmptyVec::new(x, xs.clone());
          let f = |v: NonEmptyVec<i32>| {
              let vec = v.to_vec();
              let len = vec.len() as i64;
              let sum: i64 = vec.into_iter().map(|x| x as i64).sum();
              (sum / len) as i32
          };

          let extended = <NonEmptyVec<i32> as Comonad<i32>>::extend(nev.clone(), f);
          let duplicated = <NonEmptyVec<i32> as Comonad<i32>>::duplicate(nev.clone());
          let mapped = duplicated.map(f);

          assert_eq!(extended, mapped);
      }

      #[test]
      fn test_comonad_laws(x: i32, xs in prop::collection::vec(any::<i32>(), 0..5)) {
          let nev = NonEmptyVec::new(x, xs.clone());

          // 1. Left identity: extract . duplicate = id
          let duplicated = <NonEmptyVec<i32> as Comonad<i32>>::duplicate(nev.clone());
          assert_eq!(
              <NonEmptyVec<i32> as Comonad<i32>>::extract(duplicated),
              nev
          );

          // 2. Right identity: fmap extract . duplicate = id
          let duplicated = <NonEmptyVec<i32> as Comonad<i32>>::duplicate(nev.clone());
          let mapped = duplicated.map(|w| <NonEmptyVec<i32> as Comonad<i32>>::extract(w));
          assert_eq!(mapped, nev);

          // 3. Associativity: duplicate . duplicate = fmap duplicate . duplicate
          let lhs = <NonEmptyVec<i32> as Comonad<i32>>::duplicate(
              <NonEmptyVec<i32> as Comonad<i32>>::duplicate(nev.clone())
          );
          let rhs = <NonEmptyVec<i32> as Comonad<i32>>::duplicate(nev.clone())
              .map(|w| <NonEmptyVec<i32> as Comonad<i32>>::duplicate(w));
          assert_eq!(lhs, rhs);
      }
  }
}
