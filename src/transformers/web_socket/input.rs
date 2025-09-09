use crate::transformers::web_socket::web_socket_transformer::{WebSocketMessage, WebSocketTransformer};
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for WebSocketTransformer {
  type Input = WebSocketMessage;
  type InputStream = Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>>;
}
