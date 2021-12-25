use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse::Parser, parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn adapter(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    if let syn::Data::Struct(ref mut struct_data) = &mut ast.data {
        let struct_name = ast.ident.clone();
        let field_name: syn::Member;

        match &mut struct_data.fields {
            syn::Fields::Named(fields) => {
                field_name = syn::Member::Named(syn::Ident::new(
                    "__gateway_addon_rust_adapter_handle",
                    proc_macro2::Span::call_site(),
                ));
                fields.named.push(
                    syn::Field::parse_named
                        .parse2(quote! { #field_name: gateway_addon_rust::AdapterHandle })
                        .unwrap(),
                );
            }
            syn::Fields::Unnamed(fields) => {
                field_name = syn::Member::Unnamed(syn::Index {
                    index: fields.unnamed.len() as _,
                    span: proc_macro2::Span::call_site(),
                });
                fields.unnamed.push(
                    syn::Field::parse_unnamed
                        .parse2(quote! { gateway_addon_rust::AdapterHandle })
                        .unwrap(),
                );
            }
            syn::Fields::Unit => {
                field_name = syn::Member::Unnamed(syn::Index {
                    index: 0,
                    span: proc_macro2::Span::call_site(),
                });
                let input = quote!(
                    struct #struct_name (gateway_addon_rust::AdapterHandle);
                )
                .into();
                ast = parse_macro_input!(input as DeriveInput);
            }
        }

        quote! {
            #ast
            impl gateway_addon_rust::adapter::AdapterHandleWrapper for #struct_name {
                fn adapter_handle(&self) -> &gateway_addon_rust::AdapterHandle {
                    &self.#field_name
                }
                fn adapter_handle_mut(&mut self) -> &mut gateway_addon_rust::AdapterHandle {
                    &mut self.#field_name
                }
            }
        }
        .into()
    } else {
        panic!("`adapter` has to be used with structs ")
    }
}
