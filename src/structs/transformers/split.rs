use crate::traits::transformer::TransformerConfig;

#[derive(Clone)]
pub struct SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub predicate: F,
  pub _phantom: std::marker::PhantomData<T>,
  pub config: TransformerConfig<Vec<T>>,
}
