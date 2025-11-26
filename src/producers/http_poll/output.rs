use crate::output::Output;
use crate::producers::http_poll::http_poll_producer::HttpPollProducer;
use futures::Stream;
use std::pin::Pin;

impl Output for HttpPollProducer {
  type Output = crate::producers::http_poll::http_poll_producer::HttpPollResponse;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
