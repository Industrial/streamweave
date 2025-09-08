use crate::traits::transformer::TransformerConfig;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct DistinctTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
