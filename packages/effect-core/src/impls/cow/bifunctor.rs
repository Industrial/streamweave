use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::borrow::Cow;
use std::marker::PhantomData;

// For Cow, we need to think of it as a bifunctor where:
// - The first type parameter relates to the 'borrowed' variant
// - The second type parameter relates to the 'owned' variant
// In practice, these are the same type, but conceptually we're mapping over
// either the borrowed or owned variants differently

// A mapper that applies different functions based on whether the value is borrowed or owned
#[derive(Clone)]
pub struct CowMapper<'a, B, F, G>
where
  B: Clone + 'a,
  F: FnMut(&B) -> B + CloneableThreadSafe,
  G: FnMut(&B) -> B + CloneableThreadSafe,
{
  f: F, // Function for mapping borrowed values
  g: G, // Function for mapping owned values
  _phantom: PhantomData<&'a B>,
}

impl<'a, B, F, G> CowMapper<'a, B, F, G>
where
  B: Clone + 'a,
  F: FnMut(&B) -> B + CloneableThreadSafe,
  G: FnMut(&B) -> B + CloneableThreadSafe,
{
  pub fn new(f: F, g: G) -> Self {
    CowMapper {
      f,
      g,
      _phantom: PhantomData,
    }
  }

  pub fn map_cow(&mut self, cow: Cow<'a, B>) -> Cow<'a, B> {
    match cow {
      Cow::Borrowed(b) => Cow::Owned((self.f)(b)),
      Cow::Owned(o) => Cow::Owned((self.g)(&o)),
    }
  }
}

// Extension trait for more ergonomic mapping on Cow
pub trait CowBifunctorExt<'a, B>
where
  B: Clone + 'a,
{
  fn bimap<F, G>(self, f: F, g: G) -> Cow<'a, B>
  where
    F: FnMut(&B) -> B + CloneableThreadSafe,
    G: FnMut(&B) -> B + CloneableThreadSafe;

  fn map_borrowed<F>(self, f: F) -> Cow<'a, B>
  where
    F: FnMut(&B) -> B + CloneableThreadSafe;

  fn map_owned<G>(self, g: G) -> Cow<'a, B>
  where
    G: FnMut(&B) -> B + CloneableThreadSafe;
}

// Implement the extension trait for Cow
impl<'a, B> CowBifunctorExt<'a, B> for Cow<'a, B>
where
  B: Clone + 'a,
{
  fn bimap<F, G>(self, f: F, g: G) -> Cow<'a, B>
  where
    F: FnMut(&B) -> B + CloneableThreadSafe,
    G: FnMut(&B) -> B + CloneableThreadSafe,
  {
    let mut mapper = CowMapper::new(f, g);
    mapper.map_cow(self)
  }

  fn map_borrowed<F>(self, f: F) -> Cow<'a, B>
  where
    F: FnMut(&B) -> B + CloneableThreadSafe,
  {
    let id_g = |b: &B| b.clone();
    self.bimap(f, id_g)
  }

  fn map_owned<G>(self, g: G) -> Cow<'a, B>
  where
    G: FnMut(&B) -> B + CloneableThreadSafe,
  {
    let id_f = |b: &B| b.clone();
    self.bimap(id_f, g)
  }
}

// This is a phantom type that represents the static Bifunctor instance for Cow types
#[derive(Clone)]
pub struct CowBifunctor<B1, B2>(PhantomData<(B1, B2)>);

// Implement Bifunctor for the static phantom type
impl<B1, B2> Bifunctor<B1, B2> for CowBifunctor<B1, B2>
where
  B1: Clone + CloneableThreadSafe + 'static,
  B2: Clone + CloneableThreadSafe + 'static,
{
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = CowBifunctor<C, D>;

  fn bimap<C, D, F, G>(self, _f: F, _g: G) -> Self::HigherSelf<C, D>
  where
    F: FnMut(&B1) -> C + CloneableThreadSafe,
    G: FnMut(&B2) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    // This doesn't do actual mapping - it just returns a new phantom type
    CowBifunctor(PhantomData)
  }

  fn first<C, F>(self, _f: F) -> Self::HigherSelf<C, B2>
  where
    F: FnMut(&B1) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    CowBifunctor(PhantomData)
  }

  fn second<D, G>(self, _g: G) -> Self::HigherSelf<B1, D>
  where
    G: FnMut(&B2) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    CowBifunctor(PhantomData)
  }
}

// Helper function to create a CowBifunctor
pub fn cow_bifunctor<B1, B2>() -> CowBifunctor<B1, B2>
where
  B1: Clone + CloneableThreadSafe + 'static,
  B2: Clone + CloneableThreadSafe + 'static,
{
  CowBifunctor(PhantomData)
}

// This adapter function maps a Cow using separate functions for borrowed and owned values
pub fn map_cow<'a, B, F, G>(cow: Cow<'a, B>, mut f: F, mut g: G) -> Cow<'a, B>
where
  B: Clone + 'a,
  F: FnMut(&B) -> B + CloneableThreadSafe,
  G: FnMut(&B) -> B + CloneableThreadSafe,
{
  match cow {
    Cow::Borrowed(b) => Cow::Owned(f(b)),
    Cow::Owned(o) => Cow::Owned(g(&o)),
  }
}

// Helper function to convert between different Cow types
pub fn convert_cow<'a, 'b, A, B, F>(cow: Cow<'a, A>, mut f: F) -> Cow<'b, B>
where
  A: Clone + 'a,
  B: Clone + 'b,
  F: FnMut(&A) -> B + CloneableThreadSafe,
{
  match cow {
    Cow::Borrowed(a) => Cow::Owned(f(a)),
    Cow::Owned(a) => Cow::Owned(f(&a)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::borrow::Cow;

  #[test]
  fn test_cow_bimap_borrowed() {
    let value: &String = &"hello".to_string();
    let cow: Cow<String> = Cow::Borrowed(value);

    let result = map_cow(
      cow,
      |s: &String| s[0..1].to_string(),
      |s: &String| s.to_uppercase(),
    );

    // Since our input was borrowed, the f function should be applied
    assert_eq!(result, Cow::<String>::Owned("h".to_string()));
  }

  #[test]
  fn test_cow_bimap_owned() {
    let value = "hello".to_string();
    let cow: Cow<String> = Cow::Owned(value);

    let result = map_cow(
      cow,
      |s: &String| s[0..1].to_string(),
      |s: &String| s.to_uppercase(),
    );

    // Since our input was owned, the g function should be applied
    assert_eq!(result, Cow::<String>::Owned("HELLO".to_string()));
  }

  #[test]
  fn test_cow_map_borrowed() {
    let value: &String = &"hello".to_string();
    let cow: Cow<String> = Cow::Borrowed(value);

    let result = map_cow(
      cow,
      |s: &String| s[0..1].to_string(),
      |_: &String| "unused".to_string(), // This won't be used for borrowed values
    );

    // Should apply the function to borrowed value
    assert_eq!(result, Cow::<String>::Owned("h".to_string()));
  }

  #[test]
  fn test_cow_map_owned() {
    let value = "hello".to_string();
    let cow: Cow<String> = Cow::Owned(value);

    let result = map_cow(
      cow,
      |_: &String| "unused".to_string(), // This won't be used for owned values
      |s: &String| s.to_uppercase(),
    );

    // Should apply the function to owned value
    assert_eq!(result, Cow::<String>::Owned("HELLO".to_string()));
  }

  #[test]
  fn test_direct_map_cow() {
    let value: &String = &"hello".to_string();
    let cow: Cow<String> = Cow::Borrowed(value);

    let result = map_cow(
      cow,
      |s: &String| s[0..1].to_string(),
      |s: &String| s.to_uppercase(),
    );

    assert_eq!(result, Cow::<String>::Owned("h".to_string()));
  }

  #[test]
  fn test_cow_conversion() {
    let value: &String = &"123".to_string();
    let cow: Cow<String> = Cow::Borrowed(value);

    // Convert String to Vec<u8>
    let result = convert_cow(cow, |s: &String| s.as_bytes().to_vec());

    assert_eq!(result, Cow::<Vec<u8>>::Owned(vec![49u8, 50u8, 51u8]));
  }
}
