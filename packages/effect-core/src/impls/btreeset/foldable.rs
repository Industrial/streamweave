// Commenting out implementation as it requires Functor and Category traits

/* Commenting out the Foldable implementation for BTreeSet as it requires the Functor trait
   which in turn requires the Category trait, but there's no Category implementation for BTreeSet.

impl<T: Ord + CloneableThreadSafe> Foldable<T> for BTreeSet<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    let mut acc = init;
    for x in self {
      acc = f(acc, &x);
    }
    acc
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // For BTreeSet, fold_right naturally uses the reverse iteration order
    let entries: Vec<T> = self.into_iter().collect();
    let mut acc = init;
    for x in entries.into_iter().rev() {
      acc = f(&x, acc);
    }
    acc
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let mut iter = self.into_iter();
    let first = iter.next().unwrap();
    let mut acc = first;

    for x in iter {
      acc = f(&acc, &x);
    }

    Some(acc)
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    // For BTreeSet, reduce_right uses the reverse order
    let entries: Vec<T> = self.into_iter().collect();
    let mut iter = entries.iter().rev();

    let first = iter.next().unwrap();
    let mut acc = first.clone();

    for x in iter {
      acc = f(x, &acc);
    }

    Some(acc)
  }
}
*/
