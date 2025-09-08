use crate::traits::transformer::TransformerConfig;

#[derive(Clone)]
pub struct SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub index: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}
