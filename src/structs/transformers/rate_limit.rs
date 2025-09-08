use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::time::Duration;

pub struct RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub rate_limit: usize,
  pub time_window: Duration,
  pub count: Arc<AtomicUsize>,
  pub window_start: Arc<tokio::sync::RwLock<tokio::time::Instant>>,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}
