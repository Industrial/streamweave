use crate::traits::transformer::TransformerConfig;

#[derive(Clone)]
pub struct TakeTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  pub take: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}
