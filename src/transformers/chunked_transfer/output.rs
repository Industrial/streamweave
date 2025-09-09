use crate::transformers::chunked_transfer::chunked_transfer_transformer::ChunkedTransferTransformer;
use crate::output::Output;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

impl Output for ChunkedTransferTransformer {
  type Output = Bytes;
  type OutputStream = Pin<Box<dyn Stream<Item = Bytes> + Send>>;
}
