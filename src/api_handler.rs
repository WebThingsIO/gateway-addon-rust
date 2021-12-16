/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WebthingsIO API Handlers.

use as_any::{AsAny, Downcast};
use async_trait::async_trait;
/// An [ApiHandler](crate::api_handler::ApiHandler) request.
pub use webthings_gateway_ipc_types::Request as ApiRequest;
/// An [ApiHandler](crate::api_handler::ApiHandler) response.
pub use webthings_gateway_ipc_types::Response as ApiResponse;

/// A trait used to specify the behaviour of a WebthingsIO API Handlers.
///
/// An API Handler allows you to provide custom routes at `/extensions/<plugin-id>/api/`.
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{
/// #     prelude::*, plugin::connect, example::ExampleDeviceBuilder,
/// #     api_handler::{ApiHandler, ApiRequest, ApiResponse}
/// # };
/// # use async_trait::async_trait;
/// # use serde_json::json;
/// struct ExampleApiHandler();
///
/// #[async_trait]
/// impl ApiHandler for ExampleApiHandler {
///     async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String> {
///         match request.path.as_ref() {
///             "/example-route" => Ok(ApiResponse {
///                 content: json!("foo"),
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
/// #       Self()
/// #   }
/// # }
/// #
/// # #[tokio::main]
/// pub async fn main() -> Result<(), ApiError> {
///     let mut plugin = connect("example-addon").await?;
///     plugin.set_api_handler(ExampleApiHandler::new()).await?;
///     plugin.event_loop().await;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait ApiHandler: Send + Sync + AsAny + 'static {
    /// Called when this API Handler should be unloaded.
    async fn on_unload(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Called when a route at `/extensions/<plugin-id>/api/` was requested.
    async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String>;
}

impl Downcast for dyn ApiHandler {}

pub(crate) struct NoopApiHandler;

impl NoopApiHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiHandler for NoopApiHandler {
    async fn handle_request(&mut self, _request: ApiRequest) -> Result<ApiResponse, String> {
        Err("No Api Handler registered".to_owned())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::api_handler::{ApiHandler, ApiRequest, ApiResponse};
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        pub ApiHandlerHelper {
            pub async fn on_unload(&mut self) -> Result<(), String>;
            pub async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String>;
        }
    }

    pub struct MockApiHandler {
        pub api_handler_helper: MockApiHandlerHelper,
    }

    impl MockApiHandler {
        pub fn new() -> Self {
            Self {
                api_handler_helper: MockApiHandlerHelper::default(),
            }
        }
    }

    #[async_trait]
    impl ApiHandler for MockApiHandler {
        async fn on_unload(&mut self) -> Result<(), String> {
            self.api_handler_helper.on_unload().await
        }

        async fn handle_request(&mut self, request: ApiRequest) -> Result<ApiResponse, String> {
            self.api_handler_helper.handle_request(request).await
        }
    }
}
