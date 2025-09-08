use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub skip: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
