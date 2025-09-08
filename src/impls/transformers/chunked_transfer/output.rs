use crate::structs::transformers::chunked_transfer::ChunkedTransferTransformer;
use crate::traits::output::Output;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

impl Output for ChunkedTransferTransformer {
  type Output = Bytes;
  type OutputStream = Pin<Box<dyn Stream<Item = Bytes> + Send>>;
}
