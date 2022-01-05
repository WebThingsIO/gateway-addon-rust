/// Use this on a struct to generate a built adapter around it, including useful impls.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::prelude::*;
/// # use async_trait::async_trait;
/// #[adapter]
/// struct ExampleAdapter {
///     foo: i32,
/// }
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
/// # use gateway_addon_rust::{prelude::*, adapter::{BuiltAdapter, AdapterBuilder}};
/// # use std::ops::{Deref, DerefMut};
/// # use async_trait::async_trait;
/// struct ExampleAdapter {
///     foo: i32,
/// }
///
/// struct BuiltExampleAdapter {
///     data: ExampleAdapter,
///     adapter_handle: AdapterHandle,
/// }
///
/// impl BuiltAdapter for BuiltExampleAdapter {
///     fn adapter_handle(&self) -> &AdapterHandle {
///         &self.adapter_handle
///     }
///     fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
///         &mut self.adapter_handle
///     }
/// }
///
/// impl AdapterBuilder for ExampleAdapter {
///     type BuiltAdapter = BuiltExampleAdapter;
///     fn build(data: Self, adapter_handle: AdapterHandle) -> Self::BuiltAdapter {
///         BuiltExampleAdapter {
///             data,
///             adapter_handle,
///         }
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
pub use gateway_addon_rust_codegen::adapter;
