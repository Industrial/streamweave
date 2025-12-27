use crate::http_poll_producer::HttpPollProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for HttpPollProducer {
  type Output = crate::http_poll_producer::HttpPollResponse;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
