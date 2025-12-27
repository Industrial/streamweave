use crate::group_by_transformer::GroupByTransformer;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;
use streamweave::Output;

impl<F, T, K> Output for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Clone + Send + Sync + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type Output = (K, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (K, Vec<T>)> + Send>>;
}
