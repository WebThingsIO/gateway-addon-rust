use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::str::FromStr;
use syn::DeriveInput;

/// Use this on a struct to generate a built adapter around it, including useful impls.
/// 
/// # Examples
/// ```
/// # use gateway_addon_rust::prelude::*;
/// # use async_trait::async_trait;
/// #[adapter]
/// struct ExampleAdapter { foo: i32 }
/// 
/// #[async_trait]
/// impl Adapter for BuiltExampleAdapter {
///     async fn on_unload(&mut self) -> Result<(), String> {
///         println!("Foo: {}", self.foo);
///         Ok(())
///     }
/// }
/// ```
/// will expand to
/// ```
/// # use gateway_addon_rust::{prelude::*, adapter::AdapterHandleWrapper};
/// # use std::ops::{Deref, DerefMut};
/// # use async_trait::async_trait;
/// struct ExampleAdapter { foo: i32 }
/// 
/// struct BuiltExampleAdapter{
///     data: ExampleAdapter,
///     adapter_handle: AdapterHandle
/// }
/// 
/// impl AdapterHandleWrapper for BuiltExampleAdapter {
///     fn adapter_handle(&self) -> &AdapterHandle {
///         &self.adapter_handle
///     }
///     fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
///         &mut self.adapter_handle
///     }
/// }
/// 
/// impl BuildAdapter for ExampleAdapter {
///     type BuiltAdapter = BuiltExampleAdapter;
///     fn build(data: Self, adapter_handle: AdapterHandle) -> Self::BuiltAdapter {
///         BuiltExampleAdapter { data, adapter_handle }
///     }
/// }
/// 
/// impl Deref for BuiltExampleAdapter {
///     type Target = ExampleAdapter;
///     fn deref(&self) -> &Self::Target {
///         &self.data
///     }
/// }
/// 
/// impl DerefMut for BuiltExampleAdapter {
///     fn deref_mut(&mut self) -> &mut Self::Target {
///         &mut self.data
///     }
/// }
/// 
/// #[async_trait]
/// impl Adapter for BuiltExampleAdapter {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn adapter(_args: TokenStream, input: TokenStream) -> TokenStream {
    apply_macro(input, "adapter", "Adapter")
}

#[proc_macro_attribute]
pub fn device(_args: TokenStream, input: TokenStream) -> TokenStream {
    apply_macro(input, "device", "Device")
}

fn apply_macro(input: TokenStream, name_snail_case: &str, name_camel_case: &str) -> TokenStream {
    if let Ok(ast) = syn::parse2::<DeriveInput>(input.into()) {
        alter_struct(ast, name_snail_case, name_camel_case).into()
    } else {
        panic!("`{}` has to be used with structs", name_snail_case)
    }
}

fn alter_struct(ast: DeriveInput, name_snail_case: &str, name_camel_case: &str) -> TokenStream2 {
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
    let struct_handle = TokenStream2::from_str(&format!(
        "gateway_addon_rust::{}::{}Handle",
        name_snail_case, name_camel_case
    ))
    .unwrap();
    let fn_handle = TokenStream2::from_str(&format!("{}_handle", name_snail_case)).unwrap();
    let fn_handle_mut = TokenStream2::from_str(&format!("{}_handle_mut", name_snail_case)).unwrap();

    let struct_name = ast.ident.clone();
    let struct_built_name = TokenStream2::from_str(&format!("Built{}", struct_name)).unwrap();

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
