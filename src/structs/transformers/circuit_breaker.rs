use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::time::Duration;

#[derive(Clone)]
pub struct CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub failure_threshold: usize,
  pub reset_timeout: Duration,
  pub failure_count: Arc<AtomicUsize>,
  pub last_failure_time: Arc<tokio::sync::RwLock<Option<tokio::time::Instant>>>,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
