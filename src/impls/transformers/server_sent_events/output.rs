use crate::structs::transformers::server_sent_events::ServerSentEventsTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for ServerSentEventsTransformer {
  type Output = crate::structs::transformers::server_sent_events::SSEMessage;
  type OutputStream = Pin<
    Box<dyn Stream<Item = crate::structs::transformers::server_sent_events::SSEMessage> + Send>,
  >;
}
