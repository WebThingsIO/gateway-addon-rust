/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    api_handler::ApiHandler,
    client::Client,
    message_handler::{MessageHandler, MessageResult},
};
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

#[async_trait]
impl MessageHandler for (Arc<Mutex<dyn ApiHandler>>, Arc<Mutex<Client>>) {
    async fn handle_message(&mut self, message: IPCMessage) -> Result<MessageResult, String> {
        let (api_handler, client) = self;
        let mut api_handler = api_handler.lock().await;
        let mut client = client.lock().await;

        match message {
            IPCMessage::ApiHandlerUnloadRequest(ApiHandlerUnloadRequest { data, .. }) => {
                log::info!("Received request to unload api handler");

                api_handler
                    .on_unload()
                    .await
                    .map_err(|err| format!("Could not unload api handler: {}", err))?;

                let message = ApiHandlerUnloadResponseMessageData {
                    plugin_id: data.plugin_id.clone(),
                    package_name: data.plugin_id.clone(),
                }
                .into();

                client
                    .send_message(&message)
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;
            }
            IPCMessage::ApiHandlerApiRequest(ApiHandlerApiRequest { data, .. }) => {
                let result = api_handler.handle_request(data.request).await;

                let response = result.clone().unwrap_or_else(|err| ApiResponse {
                    content: serde_json::Value::String(err),
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
                    .send_message(&message)
                    .await
                    .map_err(|err| format!("{:?}", err))?;

                result
                    .map_err(|err| format!("Error during api_handler.handle_request: {}", err))?;
            }
            msg => return Err(format!("Unexpected msg: {:?}", msg)),
        }
        Ok(MessageResult::Continue)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        api_handler::{tests::MockApiHandler, ApiHandler, ApiRequest, ApiResponse},
        message_handler::MessageHandler,
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
