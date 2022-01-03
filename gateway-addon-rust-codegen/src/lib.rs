use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::str::FromStr;
use syn::DeriveInput;

#[proc_macro_attribute]
pub fn adapter(_args: TokenStream, input: TokenStream) -> TokenStream {
    apply_macro(input, "adapter", "Adapter", None)
}

#[proc_macro_attribute]
pub fn device(_args: TokenStream, input: TokenStream) -> TokenStream {
    apply_macro(input, "device", "Device", None)
}

#[proc_macro_attribute]
pub fn property(_args: TokenStream, input: TokenStream) -> TokenStream {
    apply_macro(input, "property", "Property", Some("Value"))
}

fn apply_macro(
    input: TokenStream,
    name_snail_case: &str,
    name_camel_case: &str,
    generic_name: Option<&str>,
) -> TokenStream {
    if let Ok(ast) = syn::parse2::<DeriveInput>(input.into()) {
        alter_struct(ast, name_snail_case, name_camel_case, generic_name).into()
    } else {
        panic!("`{}` has to be used with structs", name_snail_case)
    }
}

fn alter_struct(
    ast: DeriveInput,
    name_snail_case: &str,
    name_camel_case: &str,
    generic_name: Option<&str>,
) -> TokenStream2 {
    let struct_name = ast.ident.clone();
    let struct_built_name = TokenStream2::from_str(&format!("Built{}", struct_name)).unwrap();

    let trait_handle_wrapper = TokenStream2::from_str(&format!(
        "gateway_addon_rust::{}::{}HandleWrapper",
        name_snail_case, name_camel_case
    ))
    .unwrap();
    let trait_build = TokenStream2::from_str(&format!(
        "gateway_addon_rust::{}::Build{}",
        name_snail_case, name_camel_case
    ))
    .unwrap();
    let struct_built = TokenStream2::from_str(&format!("Built{}", name_camel_case)).unwrap();
    let struct_handle = TokenStream2::from_str(&if let Some(generic_name) = generic_name {
        format!(
            "gateway_addon_rust::{name_snail_case}::{name_camel_case}Handle<<{struct_name} as gateway_addon_rust::{name_snail_case}::{name_camel_case}Structure>::{generic_name}>",
            name_snail_case = name_snail_case,
            name_camel_case = name_camel_case,
            struct_name = struct_name,
            generic_name = generic_name,
        )
    } else {
        format!(
            "gateway_addon_rust::{}::{}Handle",
            name_snail_case, name_camel_case
        )
    })
    .unwrap();
    let fn_handle = TokenStream2::from_str(&format!("{}_handle", name_snail_case)).unwrap();
    let fn_handle_mut = TokenStream2::from_str(&format!("{}_handle_mut", name_snail_case)).unwrap();
    let typedef = TokenStream2::from_str(&if let Some(generic_name) = generic_name {
        format!(
            "type {generic_name} = <{struct_name} as gateway_addon_rust::{name_snail_case}::{name_camel_case}Structure>::{generic_name};",
            name_snail_case = name_snail_case,
            name_camel_case = name_camel_case,
            struct_name = struct_name,
            generic_name = generic_name,
        )
    } else {
        "".to_owned()
    })
    .unwrap();

    quote! {
        #ast
        impl #trait_build for #struct_name {
            type #struct_built = #struct_built_name;
            fn build(data: Self, #fn_handle: #struct_handle) -> Self::#struct_built {
                #struct_built_name { data, #fn_handle }
            }
        }
        struct #struct_built_name {
            data: #struct_name,
            #fn_handle: #struct_handle,
        }
        impl #trait_handle_wrapper for #struct_built_name {
            #typedef
            fn #fn_handle(&self) -> &#struct_handle {
                &self.#fn_handle
            }
            fn #fn_handle_mut(&mut self) -> &mut #struct_handle {
                &mut self.#fn_handle
            }
        }
        impl std::ops::Deref for #struct_built_name {
            type Target = #struct_name;
            fn deref(&self) -> &Self::Target {
                &self.data
            }
        }
        impl std::ops::DerefMut for #struct_built_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.data
            }
        }
    }
}
