/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WebthingsIO API Handlers.

use crate::client::Client;
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
/// An [ApiHandler](crate::api_handler::ApiHandler) request.
pub use webthings_gateway_ipc_types::Request as ApiRequest;
/// An [ApiHandler](crate::api_handler::ApiHandler) response.
pub use webthings_gateway_ipc_types::Response as ApiResponse;
use webthings_gateway_ipc_types::{
    ApiHandlerApiRequest, ApiHandlerApiResponseMessageData, ApiHandlerUnloadRequest,
    ApiHandlerUnloadResponseMessageData, Message as IPCMessage,
};

/// A trait used to specify the behaviour of a WebthingsIO API Handlers.
///
/// An API Handler allows you to provide custom routes at `/extensions/<plugin-id>/api/`.
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{
/// #     prelude::*, plugin::connect, example::ExampleDeviceBuilder,
/// #     api_handler::{ApiHandler, ApiRequest, ApiResponse}, error::WebthingsError
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
/// pub async fn main() -> Result<(), WebthingsError> {
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

pub(crate) async fn handle_message(
    api_handler: Arc<Mutex<dyn ApiHandler>>,
    client: Arc<Mutex<Client>>,
    message: IPCMessage,
) -> Result<(), String> {
    match message {
        IPCMessage::ApiHandlerUnloadRequest(ApiHandlerUnloadRequest { data, .. }) => {
            log::info!("Received request to unload api handler");

            api_handler
                .lock()
                .await
                .on_unload()
                .await
                .map_err(|err| format!("Could not unload api handler: {}", err))?;

            let message = ApiHandlerUnloadResponseMessageData {
                plugin_id: data.plugin_id.clone(),
                package_name: data.plugin_id.clone(),
            }
            .into();

            client
                .lock()
                .await
                .send_message(&message)
                .await
                .map_err(|err| format!("Could not send unload response: {}", err))?;

            Ok(())
        }
        IPCMessage::ApiHandlerApiRequest(ApiHandlerApiRequest { data, .. }) => {
            let result = api_handler.lock().await.handle_request(data.request).await;

            let response = result.clone().unwrap_or_else(|err| ApiResponse {
                content: serde_json::Value::String(err.clone()),
                content_type: json!("text/plain"),
                status: 500,
            });
            let message = ApiHandlerApiResponseMessageData {
                message_id: data.message_id,
                package_name: data.plugin_id.clone(),
                plugin_id: data.plugin_id.clone(),
                response,
            }
            .into();

            client
                .lock()
                .await
                .send_message(&message)
                .await
                .map_err(|err| format!("{:?}", err))?;

            result.map_err(|err| format!("Error during api_handler.handle_request: {}", err))?;
            Ok(())
        }
        msg => Err(format!("Unexpected msg: {:?}", msg)),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        api_handler::{ApiHandler, ApiRequest, ApiResponse},
        plugin::tests::{plugin, set_mock_api_handler},
        Plugin,
    };
    use as_any::Downcast;
    use async_trait::async_trait;
    use mockall::mock;
    use rstest::rstest;
    use serde_json::json;
    use std::collections::BTreeMap;
    use webthings_gateway_ipc_types::{
        ApiHandlerApiRequestMessageData, ApiHandlerUnloadRequestMessageData, Message,
    };

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

    const PLUGIN_ID: &str = "plugin_id";

    #[rstest]
    #[tokio::test]
    async fn test_request_api_handler_unload(mut plugin: Plugin) {
        set_mock_api_handler(&mut plugin).await;

        let message: Message = ApiHandlerUnloadRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            package_name: PLUGIN_ID.to_owned(),
        }
        .into();

        plugin
            .api_handler
            .lock()
            .await
            .downcast_mut::<MockApiHandler>()
            .unwrap()
            .api_handler_helper
            .expect_on_unload()
            .times(1)
            .returning(|| Ok(()));

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::ApiHandlerUnloadResponse(msg) => msg.data.plugin_id == PLUGIN_ID,
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_api_handler_handle_request(mut plugin: Plugin) {
        set_mock_api_handler(&mut plugin).await;

        let request = ApiRequest {
            body: BTreeMap::new(),
            method: "GET".to_owned(),
            path: "/".to_string(),
            query: BTreeMap::new(),
        };
        let expected_response = ApiResponse {
            content: json!("foo"),
            content_type: json!("text/plain"),
            status: 200,
        };
        let message_id = 42;

        let message: Message = ApiHandlerApiRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            package_name: PLUGIN_ID.to_owned(),
            message_id,
            request: request.clone(),
        }
        .into();

        let expected_response_clone = expected_response.clone();
        plugin
            .api_handler
            .lock()
            .await
            .downcast_mut::<MockApiHandler>()
            .unwrap()
            .api_handler_helper
            .expect_handle_request()
            .withf(move |req| req.method == request.method && req.path == request.path)
            .times(1)
            .returning(move |_| Ok(expected_response_clone.clone()));

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::ApiHandlerApiResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID && msg.data.response == expected_response
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }
}
