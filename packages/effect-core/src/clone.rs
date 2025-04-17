use std::marker::PhantomData;

use crate::{compose::Compose, morphism::Morphism};

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for Morphism<A, B> {
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for Compose<A, B> {
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      g: self.g.clone(),
    }
  }
}
