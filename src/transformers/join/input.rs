use crate::input::Input;
use crate::transformers::join::join_transformer::JoinTransformer;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<L, R, K, LF, RF> Input for JoinTransformer<L, R, K, LF, RF>
where
  L: std::fmt::Debug + Clone + Send + Sync + 'static,
  R: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: Hash + Eq + Clone + Send + Sync + 'static,
  LF: Fn(&L) -> K + Clone + Send + Sync + 'static,
  RF: Fn(&R) -> K + Clone + Send + Sync + 'static,
{
  type Input = L;
  type InputStream = Pin<Box<dyn Stream<Item = L> + Send>>;
}
