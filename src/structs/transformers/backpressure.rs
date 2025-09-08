use crate::traits::transformer::TransformerConfig;

pub struct BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub buffer_size: usize,
  pub config: TransformerConfig<T>,
}
