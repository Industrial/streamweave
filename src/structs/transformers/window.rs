use crate::traits::transformer::TransformerConfig;

#[derive(Clone)]
pub struct WindowTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  pub size: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}
