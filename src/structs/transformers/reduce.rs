use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  pub accumulator: Acc,
  pub reducer: F,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
