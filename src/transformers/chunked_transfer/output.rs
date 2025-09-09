use crate::output::Output;
use crate::transformers::chunked_transfer::chunked_transfer_transformer::ChunkedTransferTransformer;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

impl Output for ChunkedTransferTransformer {
  type Output = Bytes;
  type OutputStream = Pin<Box<dyn Stream<Item = Bytes> + Send>>;
}
