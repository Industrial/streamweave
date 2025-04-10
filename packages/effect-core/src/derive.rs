use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, Type, parse_macro_input};

#[proc_macro_derive(Functor, attributes(map))]
pub fn derive_functor(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
    impl #impl_generics Functor<A> for #name #ty_generics #where_clause {
      type HigherSelf<U> = #name<U>;

      fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
      where
        F: FnMut(A) -> U,
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
    impl #impl_generics Monad<A> for #name #ty_generics #where_clause {
      type SelfTrait<U> = #name<U>;
      type Unit<U> = #name<U>;
      type Bind<U, F> = #name<U>;

      fn unit(a: A) -> Self::Unit<A> {
        #name::unit(a)
      }

      fn bind<B, MB: Id<#name<B>>, F>(self, f: F) -> Self::Bind<B, F>
      where
        F: FnOnce(A) -> #name<B>,
      {
        self.bind(f)
      }
    }
  };

  TokenStream::from(expanded)
}
