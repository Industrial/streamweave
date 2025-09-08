use crate::traits::consumer::ConsumerConfig;
use tokio::sync::mpsc::Sender;

pub struct ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub channel: Option<Sender<T>>,
  pub config: ConsumerConfig<T>,
}
