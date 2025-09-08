use crate::error::ErrorStrategy;
use crate::structs::transformers::rate_limit::RateLimitTransformer;
use crate::traits::transformer::{Transformer, TransformerConfig};
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::time::Duration;

impl<T> RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      rate_limit,
      time_window,
      count: Arc::new(AtomicUsize::new(0)),
      window_start: Arc::new(tokio::sync::RwLock::new(tokio::time::Instant::now())),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  pub async fn _check_rate_limit(&self) -> Result<(), crate::error::StreamError<T>> {
    let now = tokio::time::Instant::now();
    let mut window_start = self.window_start.write().await;

    if now.duration_since(*window_start) >= self.time_window {
      self.count.store(0, std::sync::atomic::Ordering::SeqCst);
      *window_start = now;
    }

    if self.count.load(std::sync::atomic::Ordering::SeqCst) >= self.rate_limit {
      Err(crate::error::StreamError::new(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::Other,
          "Rate limit exceeded",
        )),
        crate::error::ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: self.component_info().name,
          component_type: std::any::type_name::<Self>().to_string(),
        },
        self.component_info(),
      ))
    } else {
      self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      Ok(())
    }
  }
}
