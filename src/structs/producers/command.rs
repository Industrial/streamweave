use crate::traits::producer::ProducerConfig;

pub struct CommandProducer {
  pub command: String,
  pub args: Vec<String>,
  pub config: ProducerConfig<String>,
}
