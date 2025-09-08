use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub _phantom: PhantomData<T>,
  pub config: TransformerConfig<Vec<T>>,
}
