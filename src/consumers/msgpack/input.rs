use crate::consumers::msgpack::msgpack_consumer::MsgPackConsumer;
use crate::input::Input;
use futures::Stream;
use serde::Serialize;
use std::pin::Pin;

impl<T> Input for MsgPackConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
