use crate::input::Input;
use crate::transformers::server_sent_events::server_sent_events_transformer::{
  SSEMessage, ServerSentEventsTransformer,
};
use futures::Stream;
use std::pin::Pin;

impl Input for ServerSentEventsTransformer {
  type Input = SSEMessage;
  type InputStream = Pin<Box<dyn Stream<Item = SSEMessage> + Send>>;
}
