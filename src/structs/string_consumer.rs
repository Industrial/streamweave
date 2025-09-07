use crate::traits::consumer::ConsumerConfig;

pub struct StringConsumer {
  pub buffer: String,
  pub config: ConsumerConfig<String>,
}
