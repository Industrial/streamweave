use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub f: F,
  pub config: TransformerConfig<I>,
  pub _phantom_i: PhantomData<I>,
  pub _phantom_o: PhantomData<O>,
}
