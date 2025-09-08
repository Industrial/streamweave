use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;
use std::time::Duration;

pub struct DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub duration: Duration,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
