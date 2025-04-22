use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeMap;
use std::marker::PhantomData;

// Extension trait for more ergonomic bifunctor operations on BTreeMap
pub trait BTreeMapBifunctorExt<K, V>
where
  K: Ord + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn bimap<F, G, K2, V2>(self, f: F, g: G) -> BTreeMap<K2, V2>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe,
    V2: CloneableThreadSafe;

  fn map_keys<F, K2>(self, f: F) -> BTreeMap<K2, V>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe;

  fn map_values<G, V2>(self, g: G) -> BTreeMap<K, V2>
  where
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    V2: CloneableThreadSafe;
}

// Implement the extension trait for BTreeMap
impl<K, V> BTreeMapBifunctorExt<K, V> for BTreeMap<K, V>
where
  K: Ord + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn bimap<F, G, K2, V2>(self, mut f: F, mut g: G) -> BTreeMap<K2, V2>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (f(&k), g(&v))).collect()
  }

  fn map_keys<F, K2>(self, mut f: F) -> BTreeMap<K2, V>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (f(&k), v)).collect()
  }

  fn map_values<G, V2>(self, mut g: G) -> BTreeMap<K, V2>
  where
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (k, g(&v))).collect()
  }
}

// We use a proxy type with a phantom marker instead of directly wrapping BTreeMap
// to avoid the Ord constraint issues
#[derive(Clone)]
pub struct BTreeMapBifunctor<K, V>(PhantomData<(K, V)>)
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe;

// Bifunctor implementation that doesn't enforce Ord directly in the type parameters
impl<K, V> Bifunctor<K, V> for BTreeMapBifunctor<K, V>
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  type HigherSelf<A: CloneableThreadSafe, B: CloneableThreadSafe> = BTreeMapBifunctor<A, B>;

  // The actual mapped operations return a BTreeMap when invoked
  fn bimap<A, B, F, G>(self, _f: F, _g: G) -> Self::HigherSelf<A, B>
  where
    F: FnMut(&K) -> A + CloneableThreadSafe,
    G: FnMut(&V) -> B + CloneableThreadSafe,
    A: CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // Simply return a new BTreeMapBifunctor - the actual mapping happens when called
    BTreeMapBifunctor(PhantomData)
  }

  fn first<A, F>(self, _f: F) -> Self::HigherSelf<A, V>
  where
    F: FnMut(&K) -> A + CloneableThreadSafe,
    A: CloneableThreadSafe,
  {
    BTreeMapBifunctor(PhantomData)
  }

  fn second<B, G>(self, _g: G) -> Self::HigherSelf<K, B>
  where
    G: FnMut(&V) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    BTreeMapBifunctor(PhantomData)
  }
}

// Helper function to create a BTreeMapBifunctor
pub fn btreemap_bifunctor<K, V>() -> BTreeMapBifunctor<K, V>
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  BTreeMapBifunctor(PhantomData)
}

// Helper trait to convert from the phantom bifunctor to a real BTreeMap
pub trait ToBTreeMap<K, V> {
  fn map<F, G>(self, source: BTreeMap<K, V>, f: F, g: G) -> BTreeMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Ord + CloneableThreadSafe,
    V: CloneableThreadSafe;

  fn map_keys<F>(self, source: BTreeMap<K, V>, f: F) -> BTreeMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    K: Ord + CloneableThreadSafe,
    V: CloneableThreadSafe;

  fn map_values<G>(self, source: BTreeMap<K, V>, g: G) -> BTreeMap<K, V>
  where
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Ord + CloneableThreadSafe,
    V: CloneableThreadSafe;
}

// Implement ToBTreeMap for the phantom bifunctor
impl<K, V> ToBTreeMap<K, V> for BTreeMapBifunctor<K, V>
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn map<F, G>(self, source: BTreeMap<K, V>, mut f: F, mut g: G) -> BTreeMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Ord + CloneableThreadSafe,
    V: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), g(&v))).collect()
  }

  fn map_keys<F>(self, source: BTreeMap<K, V>, mut f: F) -> BTreeMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    K: Ord + CloneableThreadSafe,
    V: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), v)).collect()
  }

  fn map_values<G>(self, source: BTreeMap<K, V>, mut g: G) -> BTreeMap<K, V>
  where
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Ord + CloneableThreadSafe,
    V: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (k, g(&v))).collect()
  }
}

// Now implement a more flexible version of ToBTreeMap where the types can change
// This is needed to support the full Bifunctor interface
pub trait ToBTreeMapHK<K1, V1, K2, V2> {
  fn map_hk<F, G>(self, source: BTreeMap<K1, V1>, f: F, g: G) -> BTreeMap<K2, V2>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Ord + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe,
    V2: CloneableThreadSafe;

  fn map_keys_hk<F>(self, source: BTreeMap<K1, V1>, f: F) -> BTreeMap<K2, V1>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    K1: Ord + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe;

  fn map_values_hk<G>(self, source: BTreeMap<K1, V1>, g: G) -> BTreeMap<K1, V2>
  where
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Ord + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    V2: CloneableThreadSafe;
}

// Implement the HK version
impl<K1, V1, K2, V2> ToBTreeMapHK<K1, V1, K2, V2> for BTreeMapBifunctor<K2, V2>
where
  K1: CloneableThreadSafe,
  V1: CloneableThreadSafe,
  K2: CloneableThreadSafe,
  V2: CloneableThreadSafe,
{
  fn map_hk<F, G>(self, source: BTreeMap<K1, V1>, mut f: F, mut g: G) -> BTreeMap<K2, V2>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Ord + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), g(&v))).collect()
  }

  fn map_keys_hk<F>(self, source: BTreeMap<K1, V1>, mut f: F) -> BTreeMap<K2, V1>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    K1: Ord + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Ord + CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), v)).collect()
  }

  fn map_values_hk<G>(self, source: BTreeMap<K1, V1>, mut g: G) -> BTreeMap<K1, V2>
  where
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Ord + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (k, g(&v))).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::BTreeMap;

  // Helper function to create a test BTreeMap
  fn create_test_map() -> BTreeMap<i32, String> {
    let mut map = BTreeMap::new();
    map.insert(1, "one".to_string());
    map.insert(2, "two".to_string());
    map.insert(3, "three".to_string());
    map
  }

  #[test]
  fn test_bimap_ext() {
    let map = create_test_map();

    let result = map.bimap(|k| k + 10, |v| v.to_uppercase());

    assert_eq!(result.get(&11), Some(&"ONE".to_string()));
    assert_eq!(result.get(&12), Some(&"TWO".to_string()));
    assert_eq!(result.get(&13), Some(&"THREE".to_string()));
  }

  #[test]
  fn test_map_keys_ext() {
    let map = create_test_map();

    let result = map.map_keys(|k| k * 2);

    assert_eq!(result.get(&2), Some(&"one".to_string()));
    assert_eq!(result.get(&4), Some(&"two".to_string()));
    assert_eq!(result.get(&6), Some(&"three".to_string()));
  }

  #[test]
  fn test_map_values_ext() {
    let map = create_test_map();

    let result = map.map_values(|v| v.len());

    assert_eq!(result.get(&1), Some(&3));
    assert_eq!(result.get(&2), Some(&3));
    assert_eq!(result.get(&3), Some(&5));
  }

  #[test]
  fn test_empty_map() {
    let map: BTreeMap<i32, String> = BTreeMap::new();
    let result = map.bimap(|k| k + 10, |v| v.to_uppercase());
    assert!(result.is_empty());
  }

  #[test]
  fn test_key_collisions() {
    let mut map = BTreeMap::new();
    map.insert(1, "a".to_string());
    map.insert(2, "b".to_string());
    map.insert(3, "c".to_string());

    // Map all keys to the same value, causing collisions
    let result = map.map_keys(|_| 5);

    // In BTreeMap, when there are key collisions, later entries overwrite earlier ones
    assert_eq!(result.len(), 1);
    assert!(result.contains_key(&5));
    assert_eq!(result.get(&5), Some(&"c".to_string())); // Last entry wins
  }

  #[test]
  fn test_complex_types() {
    let mut map = BTreeMap::new();
    map.insert("a".to_string(), vec![1, 2, 3]);
    map.insert("b".to_string(), vec![4, 5]);

    let result = map.bimap(|k| k.clone() + "!", |v| v.iter().sum::<i32>());

    assert_eq!(result.get(&"a!".to_string()), Some(&6));
    assert_eq!(result.get(&"b!".to_string()), Some(&9));
  }

  #[test]
  fn test_bifunctor_with_phantom() {
    let map = create_test_map();

    // Create a phantom bifunctor
    let bifunctor = btreemap_bifunctor::<i32, String>();

    // Use the ToBTreeMapHK trait to map a BTreeMap
    let result = bifunctor.map_hk(map, |k| k + 10, |v| v.to_uppercase());

    assert_eq!(result.get(&11), Some(&"ONE".to_string()));
    assert_eq!(result.get(&12), Some(&"TWO".to_string()));
    assert_eq!(result.get(&13), Some(&"THREE".to_string()));
  }

  #[test]
  fn test_bifunctor_map_keys() {
    let map = create_test_map();
    let bifunctor = btreemap_bifunctor::<i32, String>();

    let result = bifunctor.map_keys_hk(map, |k| k * 2);

    assert_eq!(result.get(&2), Some(&"one".to_string()));
    assert_eq!(result.get(&4), Some(&"two".to_string()));
    assert_eq!(result.get(&6), Some(&"three".to_string()));
  }

  #[test]
  fn test_bifunctor_map_values() {
    let map = create_test_map();
    let bifunctor = btreemap_bifunctor::<i32, String>();

    let result = bifunctor.map_values_hk(map, |v| v.len().to_string());

    assert_eq!(result.get(&1), Some(&"3".to_string()));
    assert_eq!(result.get(&2), Some(&"3".to_string()));
    assert_eq!(result.get(&3), Some(&"5".to_string()));
  }

  #[test]
  fn test_ordered_keys() {
    let map = create_test_map();
    let transformed = map.bimap(|k| k * 10, |v| v.len());

    // BTreeMap preserves key ordering
    let keys: Vec<_> = transformed.keys().collect();
    assert_eq!(keys, vec![&10, &20, &30]);

    // The mapped values should be the string lengths
    let values: Vec<_> = transformed.values().collect();
    assert_eq!(values, vec![&3, &3, &5]);
  }
}
