use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
