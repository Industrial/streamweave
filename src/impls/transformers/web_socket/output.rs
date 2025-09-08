use crate::structs::transformers::web_socket::{WebSocketMessage, WebSocketTransformer};
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for WebSocketTransformer {
  type Output = WebSocketMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>>;
}
