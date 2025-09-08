use crate::traits::producer::ProducerConfig;

#[derive(Debug, Clone)]
pub struct EnvVarProducer {
  pub filter: Option<Vec<String>>,
  pub config: ProducerConfig<(String, String)>,
}
