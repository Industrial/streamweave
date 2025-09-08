use crate::traits::producer::ProducerConfig;
use num_traits::Num;

pub struct RangeProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Num + Copy + PartialOrd + 'static,
{
  pub start: T,
  pub end: T,
  pub step: T,
  pub config: ProducerConfig<T>,
}
