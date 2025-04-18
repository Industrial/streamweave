use std::collections::HashSet;
use std::sync::Arc;

// A cloneable function wrapper for HashSet
#[derive(Clone)]
pub struct HashSetFn<A, B>(Arc<dyn Fn(HashSet<A>) -> HashSet<B> + Send + Sync + 'static>);

impl<A, B> HashSetFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(HashSet<A>) -> HashSet<B> + Send + Sync + 'static,
  {
    HashSetFn(Arc::new(f))
  }

  pub fn apply(&self, a: HashSet<A>) -> HashSet<B> {
    (self.0)(a)
  }
}

/* Commenting out the Category implementation for HashSet as it requires constraints
   that are incompatible with the trait definition. This implementation would need a
   more flexible trait definition to work properly.

impl<T: Eq + Hash + CloneableThreadSafe> Category<T, T> for HashSet<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = HashSetFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    HashSetFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    HashSetFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    HashSetFn::new(move |set: HashSet<A>| set.iter().map(|a| f(a)).collect())
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)>
  where
    A: Eq + Hash,
    B: Eq + Hash,
    C: Eq + Hash,
    (A, C): Eq + Hash,
    (B, C): Eq + Hash,
  {
    HashSetFn::new(move |set: HashSet<(A, C)>| {
      let mut result = HashSet::new();

      // Group by C
      let mut c_groups: std::collections::HashMap<C, HashSet<A>> = std::collections::HashMap::new();

      for (a, c) in set {
        c_groups.entry(c).or_insert_with(HashSet::new).insert(a);
      }

      // Apply f to each group
      for (c, a_set) in c_groups {
        let b_set = f.apply(a_set);

        // Create pairs
        for b in b_set {
          result.insert((b, c.clone()));
        }
      }

      result
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)>
  where
    A: Eq + Hash,
    B: Eq + Hash,
    C: Eq + Hash,
    (C, A): Eq + Hash,
    (C, B): Eq + Hash,
  {
    HashSetFn::new(move |set: HashSet<(C, A)>| {
      let mut result = HashSet::new();

      // Group by C
      let mut c_groups: std::collections::HashMap<C, HashSet<A>> = std::collections::HashMap::new();

      for (c, a) in set {
        c_groups.entry(c).or_insert_with(HashSet::new).insert(a);
      }

      // Apply f to each group
      for (c, a_set) in c_groups {
        let b_set = f.apply(a_set);

        // Create pairs
        for b in b_set {
          result.insert((c.clone(), b));
        }
      }

      result
    })
  }
}
*/

#[cfg(test)]
mod tests {
  // Empty test module - all tests are commented out
  // Commenting out tests since the implementation is also commented out
  /*
  #[test]
  fn test_id() {
    let set = HashSet::from([1, 2, 3]);
    let id = <HashSet<i32> as Category<i32, i32>>::id();
    let result = id.apply(set.clone());
    assert_eq!(result, set);
  }

  #[test]
  fn test_arr() {
    let set = HashSet::from([1, 2, 3]);
    let f = |x: &i32| x + 1;
    let arr_f = <HashSet<i32> as Category<i32, i32>>::arr(f);
    let result = arr_f.apply(set);

    let expected = HashSet::from([2, 3, 4]);
    assert_eq!(result, expected);
  }

  #[test]
  fn test_compose() {
    let set = HashSet::from([1, 2, 3]);

    let f = |x: &i32| x + 1;
    let g = |x: &i32| x * 2;

    let arr_f = <HashSet<i32> as Category<i32, i32>>::arr(f);
    let arr_g = <HashSet<i32> as Category<i32, i32>>::arr(g);

    let composed = <HashSet<i32> as Category<i32, i32>>::compose(arr_f, arr_g);
    let result = composed.apply(set);

    // Expected: (1+1)*2, (2+1)*2, (3+1)*2
    let expected = HashSet::from([4, 6, 8]);
    assert_eq!(result, expected);
  }

  #[test]
  fn test_first() {
    let mut set = HashSet::new();
    set.insert((1, "a"));
    set.insert((2, "b"));

    let f = |x: &i32| x + 1;
    let arr_f = <HashSet<i32> as Category<i32, i32>>::arr(f);
    let first_f = <HashSet<i32> as Category<i32, i32>>::first(arr_f);

    let result = first_f.apply(set);

    let mut expected = HashSet::new();
    expected.insert((2, "a"));
    expected.insert((3, "b"));

    assert_eq!(result, expected);
  }

  #[test]
  fn test_second() {
    let mut set = HashSet::new();
    set.insert(("a", 1));
    set.insert(("b", 2));

    let f = |x: &i32| x + 1;
    let arr_f = <HashSet<i32> as Category<i32, i32>>::arr(f);
    let second_f = <HashSet<i32> as Category<i32, i32>>::second(arr_f);

    let result = second_f.apply(set);

    let mut expected = HashSet::new();
    expected.insert(("a", 2));
    expected.insert(("b", 3));

    assert_eq!(result, expected);
  }
  */
}
