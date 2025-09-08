use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;
use tokio::time::Duration;

pub struct RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub max_retries: usize,
  pub backoff: Duration,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}

impl<T> Clone for RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      max_retries: self.max_retries,
      backoff: self.backoff,
      config: self.config.clone(),
      _phantom: PhantomData,
    }
  }
}
