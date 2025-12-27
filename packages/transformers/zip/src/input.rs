use crate::zip_transformer::ZipTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for ZipTransformer<T> {
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}
