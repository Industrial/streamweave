use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Functor, attributes(map))]
pub fn derive_functor(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
    impl #impl_generics effect_core::functor::Functor<T> for #name #ty_generics #where_clause {
      type HigherSelf<U: Send + Sync + 'static> = #name<U>;

      fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
      where
        F: FnMut(T) -> B + Send + Sync + 'static,
        B: Send + Sync + 'static,
      {
        self.map(f)
      }
    }
  };

  TokenStream::from(expanded)
}

#[proc_macro_derive(Monad, attributes(unit, bind))]
pub fn derive_monad(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
    impl #impl_generics effect_core::monad::Monad<T> for #name #ty_generics #where_clause {
      type HigherSelf<U: Send + Sync + 'static> = #name<U>;

      fn pure(value: T) -> Self::HigherSelf<T>
      where
        T: Send + Sync + 'static,
      {
        #name::pure(value)
      }

      fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
      where
        F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
        B: Send + Sync + 'static,
      {
        self.bind(f)
      }
    }
  };

  TokenStream::from(expanded)
}

#[proc_macro_derive(Applicative, attributes(pure, ap))]
pub fn derive_applicative(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
    impl #impl_generics effect_core::applicative::Applicative<T> for #name #ty_generics #where_clause {
      type HigherSelf<U: Send + Sync + 'static> = #name<U>;

      fn pure(value: T) -> Self::HigherSelf<T>
      where
        T: Send + Sync + 'static,
      {
        #name::pure(value)
      }

      fn ap<B, F>(self, mut f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
      where
        F: FnMut(T) -> B + Send + Sync + 'static,
        B: Send + Sync + 'static,
      {
        self.ap(f)
      }
    }
  };

  TokenStream::from(expanded)
}

#[proc_macro_derive(Mappable)]
pub fn derive_mappable(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
    impl #impl_generics effect_core::functor::Mappable<T> for #name #ty_generics #where_clause {}
  };

  TokenStream::from(expanded)
}
