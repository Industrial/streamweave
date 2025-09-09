use crate::input::Input;
use crate::transformers::group_by::group_by_transformer::GroupByTransformer;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<F, T, K> Input for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Clone + Send + Sync + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
