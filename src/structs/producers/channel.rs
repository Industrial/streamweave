use crate::traits::producer::ProducerConfig;
use tokio::sync::mpsc;

pub struct ChannelProducer<T: std::fmt::Debug + Clone + Send + Sync> {
  pub rx: mpsc::Receiver<T>,
  pub config: ProducerConfig<T>,
}
