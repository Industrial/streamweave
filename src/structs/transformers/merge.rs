use crate::traits::transformer::TransformerConfig;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

pub struct MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub _phantom: PhantomData<T>,
  pub config: TransformerConfig<T>,
  pub streams: Vec<Pin<Box<dyn Stream<Item = T> + Send>>>,
}
