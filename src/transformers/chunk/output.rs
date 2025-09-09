use crate::output::Output;
use crate::transformers::chunk::chunk_transformer::ChunkTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}
