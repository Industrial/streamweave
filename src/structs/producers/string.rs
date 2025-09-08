use crate::traits::producer::ProducerConfig;

pub struct StringProducer {
  pub data: String,
  pub chunk_size: usize,
  pub config: ProducerConfig<String>,
}
