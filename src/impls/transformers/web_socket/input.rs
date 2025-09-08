use crate::structs::transformers::web_socket::{WebSocketMessage, WebSocketTransformer};
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for WebSocketTransformer {
  type Input = WebSocketMessage;
  type InputStream = Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>>;
}
