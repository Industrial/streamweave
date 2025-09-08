use crate::structs::transformers::server_sent_events::ServerSentEventsTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for ServerSentEventsTransformer {
  type Input = crate::structs::transformers::server_sent_events::SSEMessage;
  type InputStream = Pin<
    Box<dyn Stream<Item = crate::structs::transformers::server_sent_events::SSEMessage> + Send>,
  >;
}
