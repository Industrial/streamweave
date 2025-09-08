use crate::structs::transformers::group_by::GroupByTransformer;
use crate::traits::input::Input;
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
