use crate::transformers::server_sent_events::server_sent_events_transformer::{
  ServerSentEventsTransformer, SSEMessage,
};
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for ServerSentEventsTransformer {
  type Input = SSEMessage;
  type InputStream = Pin<
    Box<dyn Stream<Item = SSEMessage> + Send>,
  >;
}
