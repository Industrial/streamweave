use crate::traits::producer::ProducerConfig;
use std::time::Duration;

pub struct IntervalProducer {
  pub interval: Duration,
  pub config: ProducerConfig<()>,
}
