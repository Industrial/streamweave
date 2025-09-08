use crate::traits::transformer::TransformerConfig;
use futures::Stream;
use std::pin::Pin;

pub struct ConcatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub other: Pin<Box<dyn Stream<Item = T> + Send>>,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}
