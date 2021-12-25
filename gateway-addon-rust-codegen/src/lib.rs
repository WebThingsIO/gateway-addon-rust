use proc_macro::{self, TokenStream};
use quote::quote;
use std::str::FromStr;
use syn::{parse::Parser, parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn adapter(_args: TokenStream, input: TokenStream) -> TokenStream {
    alter_struct(input, "adapter", "Adapter")
}

#[proc_macro_attribute]
pub fn device(_args: TokenStream, input: TokenStream) -> TokenStream {
    alter_struct(input, "device", "Device")
}

fn alter_struct(input: TokenStream, name_snail_case: &str, name_camel_case: &str) -> TokenStream {
    let trait_handle_wrapper = proc_macro2::TokenStream::from_str(&format!(
        "gateway_addon_rust::{}::{}HandleWrapper",
        name_snail_case, name_camel_case
    ))
    .unwrap();
    let struct_handle = proc_macro2::TokenStream::from_str(&format!(
        "gateway_addon_rust::{}::{}Handle",
        name_snail_case, name_camel_case
    ))
    .unwrap();
    let fn_handle =
        proc_macro2::TokenStream::from_str(&format!("{}_handle", name_snail_case)).unwrap();
    let fn_handle_mut =
        proc_macro2::TokenStream::from_str(&format!("{}_handle_mut", name_snail_case)).unwrap();

    let mut ast = parse_macro_input!(input as DeriveInput);
    if let syn::Data::Struct(ref mut struct_data) = &mut ast.data {
        let struct_name = ast.ident.clone();
        let field_name = field_name(struct_data, name_snail_case);
        add_struct_field(struct_data, &field_name, struct_handle.clone());

        quote! {
            #ast
            impl #trait_handle_wrapper for #struct_name {
                fn #fn_handle(&self) -> &#struct_handle {
                    &self.#field_name
                }
                fn #fn_handle_mut(&mut self) -> &mut #struct_handle {
                    &mut self.#field_name
                }
            }
        }
        .into()
    } else {
        panic!("`{}` has to be used with structs", name_snail_case)
    }
}

fn field_name(struct_data: &mut syn::DataStruct, identifier: &str) -> syn::Member {
    match &mut struct_data.fields {
        syn::Fields::Named(_) => syn::Member::Named(syn::Ident::new(
            &format!("{}_handle", identifier),
            proc_macro2::Span::call_site(),
        )),
        syn::Fields::Unnamed(fields) => syn::Member::Unnamed(syn::Index {
            index: fields.unnamed.len() as _,
            span: proc_macro2::Span::call_site(),
        }),
        syn::Fields::Unit => syn::Member::Unnamed(syn::Index {
            index: 0,
            span: proc_macro2::Span::call_site(),
        }),
    }
}

fn add_struct_field(
    struct_data: &mut syn::DataStruct,
    field_name: &syn::Member,
    field_type: proc_macro2::TokenStream,
) {
    match &mut struct_data.fields {
        syn::Fields::Named(fields) => {
            fields.named.push(
                syn::Field::parse_named
                    .parse2(quote! { #field_name: #field_type })
                    .unwrap(),
            );
        }
        syn::Fields::Unnamed(fields) => {
            fields.unnamed.push(
                syn::Field::parse_unnamed
                    .parse2(quote! { #field_type })
                    .unwrap(),
            );
        }
        syn::Fields::Unit => {
            let mut fields = syn::punctuated::Punctuated::new();
            fields.push(
                syn::Field::parse_unnamed
                    .parse2(quote! { #field_type })
                    .unwrap(),
            );
            struct_data.fields = syn::Fields::Unnamed(syn::FieldsUnnamed {
                paren_token: syn::token::Paren {
                    span: proc_macro2::Span::call_site(),
                },
                unnamed: fields,
            });
        }
    }
}
