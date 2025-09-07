use crate::traits::producer::ProducerConfig;

pub struct VecProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub data: Vec<T>,
  pub config: ProducerConfig<T>,
}
