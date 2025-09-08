use crate::structs::transformers::zip::ZipTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for ZipTransformer<T> {
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}
