use crate::transformers::chunk::chunk_transformer::ChunkTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
