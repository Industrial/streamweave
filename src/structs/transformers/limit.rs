use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub limit: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
