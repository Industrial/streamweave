use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct PartitionTransformer<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub predicate: F,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
