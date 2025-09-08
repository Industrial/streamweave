use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct BufferTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub capacity: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
