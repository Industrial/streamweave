use crate::traits::transformer::TransformerConfig;
use tokio::time::Duration;

#[derive(Clone)]
pub struct TimeoutTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  pub duration: Duration,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}
