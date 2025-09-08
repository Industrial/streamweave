use crate::traits::transformer::TransformerConfig;

pub struct MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub f: F,
  pub config: TransformerConfig<I>,
  pub _phantom_i: std::marker::PhantomData<I>,
  pub _phantom_o: std::marker::PhantomData<O>,
}
