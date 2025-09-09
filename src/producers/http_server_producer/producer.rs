use super::http_server_producer::HttpServerProducer;

use crate::producer::{Producer, ProducerConfig};

impl Producer for HttpServerProducer {
  fn produce(&mut self) -> Self::OutputStream {
    // This is a simplified implementation for the trait
    // In practice, you would use the start() method
    Box::pin(futures::stream::empty())
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Self::Output> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output> {
    &mut self.config
  }
}
