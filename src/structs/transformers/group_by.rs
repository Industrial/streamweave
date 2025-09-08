use crate::traits::transformer::TransformerConfig;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  pub key_fn: F,
  pub config: TransformerConfig<T>,
  pub _phantom_t: PhantomData<T>,
  pub _phantom_k: PhantomData<K>,
}
