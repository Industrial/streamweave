use crate::traits::consumer::ConsumerConfig;
use tokio::fs::File;

pub struct FileConsumer {
  pub file: Option<File>,
  pub path: String,
  pub config: ConsumerConfig<String>,
}
