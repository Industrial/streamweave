use crate::traits::consumer::ConsumerConfig;

pub struct ArrayConsumer<T, const N: usize>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub array: [Option<T>; N],
  pub index: usize,
  pub config: ConsumerConfig<T>,
}
