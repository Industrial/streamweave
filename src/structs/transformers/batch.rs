use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub size: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
