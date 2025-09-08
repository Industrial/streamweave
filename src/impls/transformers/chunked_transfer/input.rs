use crate::structs::transformers::chunked_transfer::ChunkedTransferTransformer;
use crate::traits::input::Input;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

impl Input for ChunkedTransferTransformer {
  type Input = Bytes;
  type InputStream = Pin<Box<dyn Stream<Item = Bytes> + Send>>;
}
