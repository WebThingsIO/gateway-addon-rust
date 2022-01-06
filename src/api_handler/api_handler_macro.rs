/// Use this on a struct to generate a built API Handler around it, including useful impls.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, api_handler::{api_handler, BuiltApiHandler, ApiHandler, ApiHandlerBuilder, ApiHandlerHandle, ApiRequest, ApiResponse}};
/// # use async_trait::async_trait;
/// #[api_handler]
/// struct ExampleApiHandler {
///     foo: i32,
/// }
///
/// #[async_trait]
/// impl ApiHandler for BuiltExampleApiHandler {
///     // ...
///     # async fn handle_request(&mut self, _: ApiRequest) -> Result<ApiResponse, String> {
///     #   Err("".to_owned())
///     # }
/// }
/// ```
/// will expand to
/// ```
/// # use gateway_addon_rust::{prelude::*, api_handler::{BuiltApiHandler, ApiHandlerBuilder, ApiHandler, ApiHandlerHandle, ApiRequest, ApiResponse}};
/// # use std::ops::{Deref, DerefMut};
/// # use async_trait::async_trait;
/// struct ExampleApiHandler {
///     foo: i32,
/// }
///
/// struct BuiltExampleApiHandler {
///     data: ExampleApiHandler,
///     api_handler_handle: ApiHandlerHandle,
/// }
///
/// impl BuiltApiHandler for BuiltExampleApiHandler {
///     fn api_handler_handle(&self) -> &ApiHandlerHandle {
///         &self.api_handler_handle
///     }
///     fn api_handler_handle_mut(&mut self) -> &mut ApiHandlerHandle {
///         &mut self.api_handler_handle
///     }
/// }
///
/// impl ApiHandlerBuilder for ExampleApiHandler {
///     type BuiltApiHandler = BuiltExampleApiHandler;
///     fn build(data: Self, api_handler_handle: ApiHandlerHandle) -> Self::BuiltApiHandler {
///         BuiltExampleApiHandler {
///             data,
///             api_handler_handle,
///         }
///     }
/// }
///
/// impl Deref for BuiltExampleApiHandler {
///     type Target = ExampleApiHandler;
///     fn deref(&self) -> &Self::Target {
///         &self.data
///     }
/// }
///
/// impl DerefMut for BuiltExampleApiHandler {
///     fn deref_mut(&mut self) -> &mut Self::Target {
///         &mut self.data
///     }
/// }
///
/// #[async_trait]
/// impl ApiHandler for BuiltExampleApiHandler {
///     // ...
///     # async fn handle_request(&mut self, _: ApiRequest) -> Result<ApiResponse, String> {
///     #   Err("".to_owned())
///     # }
/// }
/// ```
pub use gateway_addon_rust_codegen::api_handler;
