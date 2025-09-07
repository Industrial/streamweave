use crate::traits::consumer::ConsumerConfig;

pub struct VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub vec: Vec<T>,
  pub config: ConsumerConfig<T>,
}
