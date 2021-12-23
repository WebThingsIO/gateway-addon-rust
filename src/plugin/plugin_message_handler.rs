/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    adapter::adapter_message_handler,
    api_handler::{self, ApiHandler},
    client::Client,
    database::Database,
    error::WebthingsError,
    message_handler::{MessageHandler, MessageResult},
    Adapter, AdapterHandle, Plugin,
};
use async_trait::async_trait;
use mockall_double::double;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, path::PathBuf, process, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use webthings_gateway_ipc_types::{
    AdapterAddedNotificationMessageData, AdapterCancelPairingCommand,
    AdapterCancelPairingCommandMessageData, AdapterRemoveDeviceRequest,
    AdapterRemoveDeviceRequestMessageData, AdapterStartPairingCommand,
    AdapterStartPairingCommandMessageData, AdapterUnloadRequest, AdapterUnloadRequestMessageData,
    ApiHandlerAddedNotificationMessageData, DeviceRemoveActionRequest,
    DeviceRemoveActionRequestMessageData, DeviceRequestActionRequest,
    DeviceRequestActionRequestMessageData, DeviceSavedNotification,
    DeviceSavedNotificationMessageData, DeviceSetPropertyCommand,
    DeviceSetPropertyCommandMessageData, Message, Message as IPCMessage,
    PluginErrorNotificationMessageData, PluginUnloadRequest, PluginUnloadResponseMessageData,
    Preferences, UserProfile,
};

#[async_trait]
impl MessageHandler for Plugin {
    async fn handle_message(&mut self, message: IPCMessage) -> Result<MessageResult, String> {
        match &message {
            IPCMessage::PluginUnloadRequest(PluginUnloadRequest { data, .. }) => {
                log::info!("Received request to unload plugin '{}'", data.plugin_id);

                self.unload()
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;

                Ok(MessageResult::Terminate)
            }
            IPCMessage::AdapterUnloadRequest(AdapterUnloadRequest {
                data: AdapterUnloadRequestMessageData { adapter_id, .. },
                ..
            })
            | IPCMessage::DeviceSavedNotification(DeviceSavedNotification {
                data: DeviceSavedNotificationMessageData { adapter_id, .. },
                ..
            })
            | IPCMessage::AdapterStartPairingCommand(AdapterStartPairingCommand {
                data: AdapterStartPairingCommandMessageData { adapter_id, .. },
                ..
            })
            | IPCMessage::AdapterCancelPairingCommand(AdapterCancelPairingCommand {
                data: AdapterCancelPairingCommandMessageData { adapter_id, .. },
                ..
            })
            | IPCMessage::AdapterRemoveDeviceRequest(AdapterRemoveDeviceRequest {
                data: AdapterRemoveDeviceRequestMessageData { adapter_id, .. },
                ..
            })
            | IPCMessage::DeviceSetPropertyCommand(DeviceSetPropertyCommand {
                data: DeviceSetPropertyCommandMessageData { adapter_id, .. },
                ..
            })
            | IPCMessage::DeviceRequestActionRequest(DeviceRequestActionRequest {
                data: DeviceRequestActionRequestMessageData { adapter_id, .. },
                ..
            })
            | IPCMessage::DeviceRemoveActionRequest(DeviceRemoveActionRequest {
                data: DeviceRemoveActionRequestMessageData { adapter_id, .. },
                ..
            }) => {
                self.borrow_adapter(adapter_id)
                    .map_err(|e| format!("{:?}", e))?
                    .lock()
                    .await
                    .handle_message(message)
                    .await
            }
            IPCMessage::ApiHandlerUnloadRequest(_) | IPCMessage::ApiHandlerApiRequest(_) => {
                (self.api_handler.clone(), self.client.clone())
                    .handle_message(message)
                    .await
            }
            msg => Err(format!("Unexpected msg: {:?}", msg)),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        adapter::tests::MockAdapter,
        api_handler::tests::MockApiHandler,
        message_handler::MessageHandler,
        plugin::{connect, tests::plugin},
        Adapter, Plugin,
    };
    use rstest::{fixture, rstest};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{Message, PluginUnloadRequestMessageData};

    const PLUGIN_ID: &str = "plugin_id";

    #[rstest]
    #[tokio::test]
    async fn test_request_unload(mut plugin: Plugin) {
        let message: Message = PluginUnloadRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::PluginUnloadResponse(msg) => msg.data.plugin_id == PLUGIN_ID,
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }
}
