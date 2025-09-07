use crate::traits::consumer::ConsumerConfig;
use tokio::process::Command;

pub struct CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  pub command: Option<Command>,
  pub config: ConsumerConfig<T>,
}
