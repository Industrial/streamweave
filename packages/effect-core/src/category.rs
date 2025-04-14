use proptest::prelude::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::marker::PhantomData;

/// The Category trait defines the basic operations of category theory.
pub trait Category {
  type A;
  type B;
  type C;
  type D;

  /// Identity morphism
  fn id<A>() -> impl Fn(A) -> A;

  /// Function composition
  fn compose<A, B, C>(f: &impl Fn(A) -> B, g: &impl Fn(B) -> C) -> impl Fn(A) -> C;

  /// Arrow construction
  fn arr<A, B>(f: impl Fn(A) -> B) -> impl Fn(A) -> B;

  /// First operation (applies function to first element of a pair)
  fn first<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((A, C)) -> (B, C);

  /// Second operation (applies function to second element of a pair)
  fn second<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((C, A)) -> (C, B);
}

/// Default implementation of Category trait
pub struct DefaultCategory;

impl Category for DefaultCategory {
  type A = ();
  type B = ();
  type C = ();
  type D = ();

  fn id<A>() -> impl Fn(A) -> A {
    |x| x
  }

  fn compose<A, B, C>(f: &impl Fn(A) -> B, g: &impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
  }

  fn arr<A, B>(f: impl Fn(A) -> B) -> impl Fn(A) -> B {
    f
  }

  fn first<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((A, C)) -> (B, C) {
    move |(x, y)| (f(x), y)
  }

  fn second<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((C, A)) -> (C, B) {
    move |(x, y)| (x, f(y))
  }
}

/// Implementation of Category for Box type
pub struct BoxCategory;

impl Category for BoxCategory {
  type A = Box<()>;
  type B = Box<()>;
  type C = Box<()>;
  type D = Box<()>;

  fn id<A>() -> impl Fn(A) -> A {
    |x| x
  }

  fn compose<A, B, C>(f: &impl Fn(A) -> B, g: &impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
  }

  fn arr<A, B>(f: impl Fn(A) -> B) -> impl Fn(A) -> B {
    f
  }

  fn first<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((A, C)) -> (B, C) {
    move |(x, y)| (f(x), y)
  }

  fn second<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((C, A)) -> (C, B) {
    move |(x, y)| (x, f(y))
  }
}

/// Implementation of Category for HashMap type
pub struct HashMapCategory<K>(PhantomData<K>);

impl<K: Eq + std::hash::Hash> Category for HashMapCategory<K> {
  type A = HashMap<K, ()>;
  type B = HashMap<K, ()>;
  type C = HashMap<K, ()>;
  type D = HashMap<K, ()>;

  fn id<A>() -> impl Fn(A) -> A {
    |x| x
  }

  fn compose<A, B, C>(f: &impl Fn(A) -> B, g: &impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
  }

  fn arr<A, B>(f: impl Fn(A) -> B) -> impl Fn(A) -> B {
    f
  }

  fn first<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((A, C)) -> (B, C) {
    move |(x, y)| (f(x), y)
  }

  fn second<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((C, A)) -> (C, B) {
    move |(x, y)| (x, f(y))
  }
}

/// Implementation of Category for BTreeMap type
pub struct BTreeMapCategory<K>(PhantomData<K>);

impl<K: Ord> Category for BTreeMapCategory<K> {
  type A = BTreeMap<K, ()>;
  type B = BTreeMap<K, ()>;
  type C = BTreeMap<K, ()>;
  type D = BTreeMap<K, ()>;

  fn id<A>() -> impl Fn(A) -> A {
    |x| x
  }

  fn compose<A, B, C>(f: &impl Fn(A) -> B, g: &impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
  }

  fn arr<A, B>(f: impl Fn(A) -> B) -> impl Fn(A) -> B {
    f
  }

  fn first<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((A, C)) -> (B, C) {
    move |(x, y)| (f(x), y)
  }

  fn second<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((C, A)) -> (C, B) {
    move |(x, y)| (x, f(y))
  }
}

/// Implementation of Category for HashSet type
pub struct HashSetCategory;

impl Category for HashSetCategory {
  type A = HashSet<()>;
  type B = HashSet<()>;
  type C = HashSet<()>;
  type D = HashSet<()>;

  fn id<A>() -> impl Fn(A) -> A {
    |x| x
  }

  fn compose<A, B, C>(f: &impl Fn(A) -> B, g: &impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
  }

  fn arr<A, B>(f: impl Fn(A) -> B) -> impl Fn(A) -> B {
    f
  }

  fn first<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((A, C)) -> (B, C) {
    move |(x, y)| (f(x), y)
  }

  fn second<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((C, A)) -> (C, B) {
    move |(x, y)| (x, f(y))
  }
}

/// Implementation of Category for BTreeSet type
pub struct BTreeSetCategory;

impl Category for BTreeSetCategory {
  type A = BTreeSet<()>;
  type B = BTreeSet<()>;
  type C = BTreeSet<()>;
  type D = BTreeSet<()>;

  fn id<A>() -> impl Fn(A) -> A {
    |x| x
  }

  fn compose<A, B, C>(f: &impl Fn(A) -> B, g: &impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
  }

  fn arr<A, B>(f: impl Fn(A) -> B) -> impl Fn(A) -> B {
    f
  }

  fn first<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((A, C)) -> (B, C) {
    move |(x, y)| (f(x), y)
  }

  fn second<A, B, C>(f: impl Fn(A) -> B) -> impl Fn((C, A)) -> (C, B) {
    move |(x, y)| (x, f(y))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

  proptest! {
    #[test]
    fn test_default_category_laws(x in any::<i32>()) {
      let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
      let g = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
      let h = |x: i32| x.checked_sub(1).unwrap_or(i32::MIN);
      let id = DefaultCategory::id::<i32>();

      // Identity laws
      let left_id = DefaultCategory::compose(&id, &f);
      let right_id = DefaultCategory::compose(&f, &id);
      prop_assert_eq!(left_id(x), f(x));
      prop_assert_eq!(right_id(x), f(x));

      // Associativity law
      let fg = DefaultCategory::compose(&f, &g);
      let gh = DefaultCategory::compose(&g, &h);
      let left_assoc = DefaultCategory::compose(&fg, &h);
      let right_assoc = DefaultCategory::compose(&f, &gh);
      prop_assert_eq!(left_assoc(x), right_assoc(x));
    }

    #[test]
    fn test_box_category_laws(x in any::<i32>()) {
      let boxed_x = Box::new(x);
      let f = |x: Box<i32>| Box::new((*x).checked_add(1).unwrap_or(i32::MAX));
      let g = |x: Box<i32>| Box::new((*x).checked_mul(2).unwrap_or(i32::MAX));
      let h = |x: Box<i32>| Box::new((*x).checked_sub(1).unwrap_or(i32::MIN));
      let id = BoxCategory::id::<Box<i32>>();

      // Identity laws
      let left_id = BoxCategory::compose(&id, &f);
      let right_id = BoxCategory::compose(&f, &id);
      prop_assert_eq!(*left_id(boxed_x.clone()), *f(boxed_x.clone()));
      prop_assert_eq!(*right_id(boxed_x.clone()), *f(boxed_x.clone()));

      // Associativity law
      let fg = BoxCategory::compose(&f, &g);
      let gh = BoxCategory::compose(&g, &h);
      let left_assoc = BoxCategory::compose(&fg, &h);
      let right_assoc = BoxCategory::compose(&f, &gh);
      prop_assert_eq!(*left_assoc(boxed_x.clone()), *right_assoc(boxed_x));
    }

    #[test]
    fn test_hashmap_category_laws(k in any::<i32>(), v in any::<i32>()) {
      let mut map1 = HashMap::new();
      map1.insert(k, v);

      let f = |mut m: HashMap<i32, i32>| { m.insert(k, v.checked_add(1).unwrap_or(i32::MAX)); m };
      let g = |mut m: HashMap<i32, i32>| { m.insert(k, v.checked_mul(2).unwrap_or(i32::MAX)); m };
      let h = |mut m: HashMap<i32, i32>| { m.insert(k, v.checked_sub(1).unwrap_or(i32::MIN)); m };
      let id = HashMapCategory::<i32>::id::<HashMap<i32, i32>>();

      // Identity laws
      let left_id = HashMapCategory::<i32>::compose(&id, &f);
      let right_id = HashMapCategory::<i32>::compose(&f, &id);
      prop_assert_eq!(left_id(map1.clone()), f(map1.clone()));
      prop_assert_eq!(right_id(map1.clone()), f(map1.clone()));

      // Associativity law
      let fg = HashMapCategory::<i32>::compose(&f, &g);
      let gh = HashMapCategory::<i32>::compose(&g, &h);
      let left_assoc = HashMapCategory::<i32>::compose(&fg, &h);
      let right_assoc = HashMapCategory::<i32>::compose(&f, &gh);
      prop_assert_eq!(left_assoc(map1.clone()), right_assoc(map1));
    }

    #[test]
    fn test_btreemap_category_laws(k in any::<i32>(), v in any::<i32>()) {
      let mut map1 = BTreeMap::new();
      map1.insert(k, v);

      let f = |mut m: BTreeMap<i32, i32>| { m.insert(k, v.checked_add(1).unwrap_or(i32::MAX)); m };
      let g = |mut m: BTreeMap<i32, i32>| { m.insert(k, v.checked_mul(2).unwrap_or(i32::MAX)); m };
      let h = |mut m: BTreeMap<i32, i32>| { m.insert(k, v.checked_sub(1).unwrap_or(i32::MIN)); m };
      let id = BTreeMapCategory::<i32>::id::<BTreeMap<i32, i32>>();

      // Identity laws
      let left_id = BTreeMapCategory::<i32>::compose(&id, &f);
      let right_id = BTreeMapCategory::<i32>::compose(&f, &id);
      prop_assert_eq!(left_id(map1.clone()), f(map1.clone()));
      prop_assert_eq!(right_id(map1.clone()), f(map1.clone()));

      // Associativity law
      let fg = BTreeMapCategory::<i32>::compose(&f, &g);
      let gh = BTreeMapCategory::<i32>::compose(&g, &h);
      let left_assoc = BTreeMapCategory::<i32>::compose(&fg, &h);
      let right_assoc = BTreeMapCategory::<i32>::compose(&f, &gh);
      prop_assert_eq!(left_assoc(map1.clone()), right_assoc(map1));
    }

    #[test]
    fn test_hashset_category_laws(x in any::<i32>()) {
      let mut set1 = HashSet::new();
      set1.insert(x);

      let f = |mut s: HashSet<i32>| { s.insert(x.checked_add(1).unwrap_or(i32::MAX)); s };
      let g = |mut s: HashSet<i32>| { s.insert(x.checked_mul(2).unwrap_or(i32::MAX)); s };
      let h = |mut s: HashSet<i32>| { s.insert(x.checked_sub(1).unwrap_or(i32::MIN)); s };
      let id = HashSetCategory::id::<HashSet<i32>>();

      // Identity laws
      let left_id = HashSetCategory::compose(&id, &f);
      let right_id = HashSetCategory::compose(&f, &id);
      prop_assert_eq!(left_id(set1.clone()), f(set1.clone()));
      prop_assert_eq!(right_id(set1.clone()), f(set1.clone()));

      // Associativity law
      let fg = HashSetCategory::compose(&f, &g);
      let gh = HashSetCategory::compose(&g, &h);
      let left_assoc = HashSetCategory::compose(&fg, &h);
      let right_assoc = HashSetCategory::compose(&f, &gh);
      prop_assert_eq!(left_assoc(set1.clone()), right_assoc(set1));
    }

    #[test]
    fn test_btreeset_category_laws(x in any::<i32>()) {
      let mut set1 = BTreeSet::new();
      set1.insert(x);

      let f = |mut s: BTreeSet<i32>| { s.insert(x.checked_add(1).unwrap_or(i32::MAX)); s };
      let g = |mut s: BTreeSet<i32>| { s.insert(x.checked_mul(2).unwrap_or(i32::MAX)); s };
      let h = |mut s: BTreeSet<i32>| { s.insert(x.checked_sub(1).unwrap_or(i32::MIN)); s };
      let id = BTreeSetCategory::id::<BTreeSet<i32>>();

      // Identity laws
      let left_id = BTreeSetCategory::compose(&id, &f);
      let right_id = BTreeSetCategory::compose(&f, &id);
      prop_assert_eq!(left_id(set1.clone()), f(set1.clone()));
      prop_assert_eq!(right_id(set1.clone()), f(set1.clone()));

      // Associativity law
      let fg = BTreeSetCategory::compose(&f, &g);
      let gh = BTreeSetCategory::compose(&g, &h);
      let left_assoc = BTreeSetCategory::compose(&fg, &h);
      let right_assoc = BTreeSetCategory::compose(&f, &gh);
      prop_assert_eq!(left_assoc(set1.clone()), right_assoc(set1));
    }

    #[test]
    fn test_first_second_laws(x in any::<i32>(), y in any::<String>()) {
      let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
      let g = |s: String| s + "!";

      // Test first operation
      let first_f = DefaultCategory::first(f);
      prop_assert_eq!(first_f((x, y.clone())), (x.checked_add(1).unwrap_or(i32::MAX), y.clone()));

      // Test second operation
      let second_g = DefaultCategory::second(g);
      prop_assert_eq!(second_g((x, y.clone())), (x, y + "!"));
    }

    #[test]
    fn test_arr_laws(x in any::<i32>()) {
      let f = |x: i32| x + 1;
      let arr_f = DefaultCategory::arr(f);
      prop_assert_eq!(arr_f(x), f(x));
    }
  }
}
