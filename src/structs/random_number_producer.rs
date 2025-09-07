use crate::traits::producer::ProducerConfig;
use std::ops::Range;
use std::time::Duration;

pub struct RandomNumberProducer {
  pub range: Range<i32>,
  pub count: Option<usize>,
  pub interval: Duration,
  pub config: ProducerConfig<i32>,
}
