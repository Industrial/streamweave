use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

// Extension trait for more ergonomic bifunctor operations on HashMap
pub trait HashMapBifunctorExt<K, V>
where
  K: Eq + Hash + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn bimap<F, G, K2, V2>(self, f: F, g: G) -> HashMap<K2, V2>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe,
    V2: CloneableThreadSafe;

  fn map_keys<F, K2>(self, f: F) -> HashMap<K2, V>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe;

  fn map_values<G, V2>(self, g: G) -> HashMap<K, V2>
  where
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    V2: CloneableThreadSafe;
}

// Implement the extension trait for HashMap
impl<K, V> HashMapBifunctorExt<K, V> for HashMap<K, V>
where
  K: Eq + Hash + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn bimap<F, G, K2, V2>(self, mut f: F, mut g: G) -> HashMap<K2, V2>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (f(&k), g(&v))).collect()
  }

  fn map_keys<F, K2>(self, mut f: F) -> HashMap<K2, V>
  where
    F: FnMut(&K) -> K2 + CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (f(&k), v)).collect()
  }

  fn map_values<G, V2>(self, mut g: G) -> HashMap<K, V2>
  where
    G: FnMut(&V) -> V2 + CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (k, g(&v))).collect()
  }
}

// A proxy type with phantom data to implement Bifunctor for HashMap
// without enforcing Eq+Hash constraints in the type parameters
#[derive(Clone)]
pub struct HashMapBifunctor<K, V>(PhantomData<(K, V)>)
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe;

// Bifunctor implementation without enforcing Eq+Hash constraints
impl<K, V> Bifunctor<K, V> for HashMapBifunctor<K, V>
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  type HigherSelf<A: CloneableThreadSafe, B: CloneableThreadSafe> = HashMapBifunctor<A, B>;

  fn bimap<A, B, F, G>(self, _f: F, _g: G) -> Self::HigherSelf<A, B>
  where
    F: FnMut(&K) -> A + CloneableThreadSafe,
    G: FnMut(&V) -> B + CloneableThreadSafe,
    A: CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // Simply return a new HashMapBifunctor - the actual mapping happens when called
    HashMapBifunctor(PhantomData)
  }

  fn first<A, F>(self, _f: F) -> Self::HigherSelf<A, V>
  where
    F: FnMut(&K) -> A + CloneableThreadSafe,
    A: CloneableThreadSafe,
  {
    HashMapBifunctor(PhantomData)
  }

  fn second<B, G>(self, _g: G) -> Self::HigherSelf<K, B>
  where
    G: FnMut(&V) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    HashMapBifunctor(PhantomData)
  }
}

// Helper function to create a HashMapBifunctor
pub fn hashmap_bifunctor<K, V>() -> HashMapBifunctor<K, V>
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  HashMapBifunctor(PhantomData)
}

// Helper trait to convert from the phantom bifunctor to a real HashMap
pub trait ToHashMap<K, V> {
  fn map<F, G>(self, source: HashMap<K, V>, f: F, g: G) -> HashMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Eq + Hash + CloneableThreadSafe,
    V: CloneableThreadSafe;

  fn map_keys<F>(self, source: HashMap<K, V>, f: F) -> HashMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    K: Eq + Hash + CloneableThreadSafe,
    V: CloneableThreadSafe;

  fn map_values<G>(self, source: HashMap<K, V>, g: G) -> HashMap<K, V>
  where
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Eq + Hash + CloneableThreadSafe,
    V: CloneableThreadSafe;
}

// Implement ToHashMap for the phantom bifunctor
impl<K, V> ToHashMap<K, V> for HashMapBifunctor<K, V>
where
  K: CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn map<F, G>(self, source: HashMap<K, V>, mut f: F, mut g: G) -> HashMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Eq + Hash + CloneableThreadSafe,
    V: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), g(&v))).collect()
  }

  fn map_keys<F>(self, source: HashMap<K, V>, mut f: F) -> HashMap<K, V>
  where
    F: FnMut(&K) -> K + CloneableThreadSafe,
    K: Eq + Hash + CloneableThreadSafe,
    V: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), v)).collect()
  }

  fn map_values<G>(self, source: HashMap<K, V>, mut g: G) -> HashMap<K, V>
  where
    G: FnMut(&V) -> V + CloneableThreadSafe,
    K: Eq + Hash + CloneableThreadSafe,
    V: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (k, g(&v))).collect()
  }
}

// Higher-kinded version for changing types
pub trait ToHashMapHK<K1, V1, K2, V2> {
  fn map_hk<F, G>(self, source: HashMap<K1, V1>, f: F, g: G) -> HashMap<K2, V2>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Eq + Hash + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe,
    V2: CloneableThreadSafe;

  fn map_keys_hk<F>(self, source: HashMap<K1, V1>, f: F) -> HashMap<K2, V1>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    K1: Eq + Hash + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe;

  fn map_values_hk<G>(self, source: HashMap<K1, V1>, g: G) -> HashMap<K1, V2>
  where
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Eq + Hash + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    V2: CloneableThreadSafe;
}

// Implement the higher-kinded version
impl<K1, V1, K2, V2> ToHashMapHK<K1, V1, K2, V2> for HashMapBifunctor<K2, V2>
where
  K1: CloneableThreadSafe,
  V1: CloneableThreadSafe,
  K2: CloneableThreadSafe,
  V2: CloneableThreadSafe,
{
  fn map_hk<F, G>(self, source: HashMap<K1, V1>, mut f: F, mut g: G) -> HashMap<K2, V2>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Eq + Hash + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), g(&v))).collect()
  }

  fn map_keys_hk<F>(self, source: HashMap<K1, V1>, mut f: F) -> HashMap<K2, V1>
  where
    F: FnMut(&K1) -> K2 + CloneableThreadSafe,
    K1: Eq + Hash + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    K2: Eq + Hash + CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (f(&k), v)).collect()
  }

  fn map_values_hk<G>(self, source: HashMap<K1, V1>, mut g: G) -> HashMap<K1, V2>
  where
    G: FnMut(&V1) -> V2 + CloneableThreadSafe,
    K1: Eq + Hash + CloneableThreadSafe,
    V1: CloneableThreadSafe,
    V2: CloneableThreadSafe,
  {
    source.into_iter().map(|(k, v)| (k, g(&v))).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  // Helper function to create a test HashMap
  fn create_test_map() -> HashMap<i32, String> {
    let mut map = HashMap::new();
    map.insert(1, "one".to_string());
    map.insert(2, "two".to_string());
    map.insert(3, "three".to_string());
    map
  }

  #[test]
  fn test_bimap() {
    let map = create_test_map();

    let result = map.bimap(|k| k + 10, |v| v.to_uppercase());

    assert_eq!(result.get(&11), Some(&"ONE".to_string()));
    assert_eq!(result.get(&12), Some(&"TWO".to_string()));
    assert_eq!(result.get(&13), Some(&"THREE".to_string()));
  }

  #[test]
  fn test_map_keys() {
    let map = create_test_map();

    let result = map.map_keys(|k| k * 2);

    assert_eq!(result.get(&2), Some(&"one".to_string()));
    assert_eq!(result.get(&4), Some(&"two".to_string()));
    assert_eq!(result.get(&6), Some(&"three".to_string()));
  }

  #[test]
  fn test_map_values() {
    let map = create_test_map();

    let result = map.map_values(|v| v.len());

    assert_eq!(result.get(&1), Some(&3));
    assert_eq!(result.get(&2), Some(&3));
    assert_eq!(result.get(&3), Some(&5));
  }

  #[test]
  fn test_empty_map() {
    let map: HashMap<i32, String> = HashMap::new();

    let result = map.bimap(|k| k + 10, |v| v.to_uppercase());

    assert!(result.is_empty());
  }

  #[test]
  fn test_key_collisions() {
    let mut map = HashMap::new();
    map.insert(1, "a".to_string());
    map.insert(2, "b".to_string());
    map.insert(3, "c".to_string());

    // Map all keys to the same value, causing collisions
    let result = map.map_keys(|_| 5);

    // In HashMap, when there are key collisions, later entries overwrite earlier ones
    // The exact value we get depends on the iteration order, which is not guaranteed
    assert_eq!(result.len(), 1);
    assert!(result.contains_key(&5));
  }

  #[test]
  fn test_complex_types() {
    let mut map = HashMap::new();
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
    let bifunctor = hashmap_bifunctor::<i32, String>();

    // Use the ToHashMapHK trait to map a HashMap
    let result = bifunctor.map_hk(map, |k| k + 10, |v| v.to_uppercase());

    assert_eq!(result.get(&11), Some(&"ONE".to_string()));
    assert_eq!(result.get(&12), Some(&"TWO".to_string()));
    assert_eq!(result.get(&13), Some(&"THREE".to_string()));
  }

  #[test]
  fn test_bifunctor_map_keys() {
    let map = create_test_map();
    let bifunctor = hashmap_bifunctor::<i32, String>();

    let result = bifunctor.map_keys_hk(map, |k| k * 2);

    assert_eq!(result.get(&2), Some(&"one".to_string()));
    assert_eq!(result.get(&4), Some(&"two".to_string()));
    assert_eq!(result.get(&6), Some(&"three".to_string()));
  }

  #[test]
  fn test_bifunctor_map_values() {
    let map = create_test_map();
    let bifunctor = hashmap_bifunctor::<i32, String>();

    let result = bifunctor.map_values_hk(map, |v| v.len().to_string());

    assert_eq!(result.get(&1), Some(&"3".to_string()));
    assert_eq!(result.get(&2), Some(&"3".to_string()));
    assert_eq!(result.get(&3), Some(&"5".to_string()));
  }
}
