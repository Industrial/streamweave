use crate::output::Output;
use crate::transformers::web_socket::web_socket_transformer::{
  WebSocketMessage, WebSocketTransformer,
};
use futures::Stream;
use std::pin::Pin;

impl Output for WebSocketTransformer {
  type Output = WebSocketMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>>;
}
