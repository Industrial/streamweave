use crate::traits::transformer::TransformerConfig;
use std::collections::HashSet;
use std::hash::Hash;

pub struct DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub seen: HashSet<T>,
  pub config: TransformerConfig<T>,
}
