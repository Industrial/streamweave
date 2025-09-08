use crate::traits::transformer::TransformerConfig;
use bytes::Bytes;

pub struct ChunkedTransferTransformer {
  pub config: TransformerConfig<Bytes>,
}
