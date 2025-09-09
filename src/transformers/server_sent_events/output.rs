use crate::output::Output;
use crate::transformers::server_sent_events::server_sent_events_transformer::{
  SSEMessage, ServerSentEventsTransformer,
};
use futures::Stream;
use std::pin::Pin;

impl Output for ServerSentEventsTransformer {
  type Output = SSEMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = SSEMessage> + Send>>;
}
