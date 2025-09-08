use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct ZipTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  pub config: TransformerConfig<Vec<T>>,
  pub _phantom: PhantomData<T>,
}
