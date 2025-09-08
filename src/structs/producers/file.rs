use crate::traits::producer::ProducerConfig;

pub struct FileProducer {
  pub path: String,
  pub config: ProducerConfig<String>,
}
