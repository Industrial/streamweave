use crate::traits::transformer::TransformerConfig;
use std::time::Duration;

pub struct DebounceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub duration: Duration,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}
