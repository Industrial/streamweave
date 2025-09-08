use crate::traits::transformer::TransformerConfig;

pub struct FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub predicate: F,
  pub _phantom: std::marker::PhantomData<T>,
  pub config: TransformerConfig<T>,
}
