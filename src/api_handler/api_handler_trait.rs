/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::api_handler::{ApiHandlerHandle, ApiRequest, ApiResponse};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;

/// A trait used to specify the behaviour of a WebthingsIO API Handlers.
///
/// An API Handler allows you to provide custom routes at `/extensions/<plugin-id>/api/`.
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{
/// #     prelude::*, plugin::connect,
/// #     api_handler::{api_handler, ApiHandler, ApiRequest, ApiResponse}, error::WebthingsError
/// # };
/// # use async_trait::async_trait;
/// # use serde_json::json;
/// #[api_handler]
/// struct ExampleApiHandler {
///     foo: i32,
/// }
///
/// #[async_trait]
/// impl ApiHandler for BuiltExampleApiHandler {
///     async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String> {
///         match request.path.as_ref() {
///             "/example-route" => Ok(ApiResponse {
///                 content: serde_json::to_value(self.foo).unwrap(),
///                 content_type: json!("text/plain"),
///                 status: 200,
///             }),
///             _ => Err("unknown route".to_owned()),
///         }
///     }
/// }
///
/// # impl ExampleApiHandler {
/// #   pub fn new() -> Self {
/// #       Self {
/// #           foo: 42,
/// #       }
/// #   }
/// # }
/// #
/// # #[tokio::main]
/// pub async fn main() -> Result<(), WebthingsError> {
///     let mut plugin = connect("example-addon").await?;
///     plugin.set_api_handler(ExampleApiHandler::new()).await?;
///     plugin.event_loop().await;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait ApiHandler: BuiltApiHandler + Send + Sync + AsAny + 'static {
    /// Called when this API Handler should be unloaded.
    async fn on_unload(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Called when a route at `/extensions/<plugin-id>/api/` was requested.
    async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String>;
}

impl Downcast for dyn ApiHandler {}

/// A trait used to wrap an [API handler handle][ApiHandlerHandle].
///
/// When you use the [api_handler][macro@crate::api_handler::api_handler] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, api_handler::{BuiltApiHandler, ApiHandlerHandle}};
/// # use async_trait::async_trait;
/// struct BuiltExampleApiHandler {
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
/// ```
pub trait BuiltApiHandler {
    /// Return a reference to the wrapped [API Handler handle][ApiHandlerHandle].
    fn api_handler_handle(&self) -> &ApiHandlerHandle;

    /// Return a mutable reference to the wrapped [API Handler handle][ApiHandlerHandle].
    fn api_handler_handle_mut(&mut self) -> &mut ApiHandlerHandle;
}

/// A trait used to build an [API Handler][ApiHandler] around a data struct and an [API Handler handle][ApiHandlerHandle].
///
/// When you use the [api_handler][macro@crate::api_handler::api_handler] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, api_handler::{BuiltApiHandler, ApiHandlerBuilder, ApiHandler, ApiHandlerHandle, ApiRequest, ApiResponse}};
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
///     // ...
/// #   fn api_handler_handle(&self) -> &ApiHandlerHandle {
/// #       &self.api_handler_handle
/// #   }
/// #   fn api_handler_handle_mut(&mut self) -> &mut ApiHandlerHandle {
/// #       &mut self.api_handler_handle
/// #   }
/// }
///
/// #[async_trait]
/// impl ApiHandler for BuiltExampleApiHandler {
///     // ...
///     # async fn handle_request(&mut self, _: ApiRequest) -> Result<ApiResponse, String> {
///     #   Err("".to_owned())
///     # }
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
/// ```
pub trait ApiHandlerBuilder {
    /// Type of [ApiHandler] to build.
    type BuiltApiHandler: ApiHandler;

    /// Build the [API Handler][ApiHandler] from a data struct and an [API Handler handle][ApiHandlerHandle].
    fn build(data: Self, api_handler_handle: ApiHandlerHandle) -> Self::BuiltApiHandler;
}

pub(crate) struct NoopApiHandler;
pub(crate) struct BuiltNoopApiHandler {
    api_handler_handle: ApiHandlerHandle,
}

impl ApiHandlerBuilder for NoopApiHandler {
    type BuiltApiHandler = BuiltNoopApiHandler;
    fn build(_data: Self, api_handler_handle: ApiHandlerHandle) -> Self::BuiltApiHandler {
        BuiltNoopApiHandler { api_handler_handle }
    }
}

impl BuiltApiHandler for BuiltNoopApiHandler {
    fn api_handler_handle(&self) -> &ApiHandlerHandle {
        &self.api_handler_handle
    }

    fn api_handler_handle_mut(&mut self) -> &mut ApiHandlerHandle {
        &mut self.api_handler_handle
    }
}

#[async_trait]
impl ApiHandler for BuiltNoopApiHandler {
    async fn handle_request(&mut self, _request: ApiRequest) -> Result<ApiResponse, String> {
        Err("No Api Handler registered".to_owned())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::api_handler::{
        ApiHandler, ApiHandlerBuilder, ApiHandlerHandle, ApiRequest, ApiResponse, BuiltApiHandler,
    };
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        pub ApiHandler{
            pub async fn on_unload(&mut self) -> Result<(), String>;
            pub async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String>;
        }
    }

    pub struct BuiltMockApiHandler {
        data: MockApiHandler,
        api_handler_handle: ApiHandlerHandle,
    }

    impl ApiHandlerBuilder for MockApiHandler {
        type BuiltApiHandler = BuiltMockApiHandler;
        fn build(data: Self, api_handler_handle: ApiHandlerHandle) -> Self::BuiltApiHandler {
            BuiltMockApiHandler {
                data,
                api_handler_handle,
            }
        }
    }

    impl BuiltApiHandler for BuiltMockApiHandler {
        fn api_handler_handle(&self) -> &ApiHandlerHandle {
            &self.api_handler_handle
        }

        fn api_handler_handle_mut(&mut self) -> &mut ApiHandlerHandle {
            &mut self.api_handler_handle
        }
    }

    impl std::ops::Deref for BuiltMockApiHandler {
        type Target = MockApiHandler;
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    impl std::ops::DerefMut for BuiltMockApiHandler {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    #[async_trait]
    impl ApiHandler for BuiltMockApiHandler {
        async fn on_unload(&mut self) -> Result<(), String> {
            self.data.on_unload().await
        }

        async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String> {
            self.data.handle_request(request).await
        }
    }
}
