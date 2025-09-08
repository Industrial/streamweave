use crate::traits::producer::ProducerConfig;

pub struct ArrayProducer<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> {
  pub array: [T; N],
  pub config: ProducerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}
