use crate::input::Input;
use crate::transformers::chunked_transfer::chunked_transfer_transformer::ChunkedTransferTransformer;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

impl Input for ChunkedTransferTransformer {
  type Input = Bytes;
  type InputStream = Pin<Box<dyn Stream<Item = Bytes> + Send>>;
}
