use crate::traits::consumer::ConsumerConfig;

pub struct ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  pub config: ConsumerConfig<T>,
}
