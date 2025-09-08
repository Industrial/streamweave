use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct SampleTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub probability: f64,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
