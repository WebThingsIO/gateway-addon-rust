/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::adapter::{Adapter, AdapterHandle};
use crate::api_error::ApiError;
use crate::client::{Client, WebsocketClient};
use crate::database::Database;
use futures::prelude::*;
use futures::stream::SplitStream;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;
use webthings_gateway_ipc_types::AdapterRemoveDeviceRequest;
use webthings_gateway_ipc_types::{
    AdapterAddedNotificationMessageData, AdapterCancelPairingCommand, AdapterStartPairingCommand,
    AdapterUnloadRequest, DeviceSavedNotification, DeviceSetPropertyCommand, Message,
    PluginErrorNotificationMessageData, PluginRegisterRequestMessageData, PluginUnloadRequest,
    PluginUnloadResponseMessageData, Preferences, UserProfile,
};
use webthings_gateway_ipc_types::{Message as IPCMessage, PluginRegisterResponseMessageData};

const GATEWAY_URL: &str = "ws://localhost:9500";
const DONT_RESTART_EXIT_CODE: i32 = 100;

#[cfg(not(test))]
pub async fn connect(plugin_id: &str) -> Result<Plugin, ApiError> {
    let url = Url::parse(GATEWAY_URL).expect("Could not parse url");

    let (socket, _) = connect_async(url).await.map_err(ApiError::Connect)?;

    let (sink, mut stream) = socket.split();
    let mut client = WebsocketClient::new(sink);

    let message: IPCMessage = PluginRegisterRequestMessageData {
        plugin_id: plugin_id.to_owned(),
    }
    .into();

    client.send_message(&message).await?;

    let PluginRegisterResponseMessageData {
        gateway_version: _,
        plugin_id: _,
        preferences,
        user_profile,
    } = loop {
        match read(&mut stream).await {
            None => {}
            Some(result) => match result {
                Ok(IPCMessage::PluginRegisterResponse(msg)) => {
                    break msg.data;
                }
                Ok(msg) => {
                    log::warn!("Received unexpected message {:?}", msg);
                }
                Err(err) => log::error!("Could not read message: {}", err),
            },
        }
    };

    Ok(Plugin {
        plugin_id: plugin_id.to_owned(),
        preferences,
        user_profile,
        client: Arc::new(Mutex::new(client)),
        stream,
        adapters: HashMap::new(),
    })
}

#[cfg(test)]
pub fn connect(plugin_id: &str) -> (Plugin, Arc<Mutex<crate::client::MockClient>>) {
    let preferences = Preferences {
        language: "en-US".to_owned(),
        units: webthings_gateway_ipc_types::Units {
            temperature: "degree celsius".to_owned(),
        },
    };
    let user_profile = UserProfile {
        addons_dir: "".to_owned(),
        base_dir: "".to_owned(),
        config_dir: "".to_owned(),
        data_dir: "".to_owned(),
        gateway_dir: "".to_owned(),
        log_dir: "".to_owned(),
        media_dir: "".to_owned(),
    };
    let client = Arc::new(Mutex::new(crate::client::MockClient::new()));
    (
        Plugin {
            plugin_id: plugin_id.to_owned(),
            preferences,
            user_profile,
            client: client.clone(),
            adapters: HashMap::new(),
        },
        client,
    )
}

async fn read(
    stream: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Option<Result<IPCMessage, String>> {
    stream.next().await.map(|result| match result {
        Ok(msg) => {
            let json = msg
                .to_text()
                .map_err(|err| format!("Could not get text message: {:?}", err))?;

            log::trace!("Received message {}", json);

            IPCMessage::from_str(json).map_err(|err| format!("Could not parse message: {:?}", err))
        }
        Err(err) => Err(err.to_string()),
    })
}

pub struct Plugin {
    pub plugin_id: String,
    pub preferences: Preferences,
    pub user_profile: UserProfile,
    client: Arc<Mutex<dyn Client>>,
    #[cfg(not(test))]
    stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    adapters: HashMap<String, Arc<Mutex<dyn Adapter>>>,
}

#[doc(hidden)]
pub(crate) enum MessageResult {
    Continue,
    Terminate,
}

impl Plugin {
    #[cfg(not(test))]
    pub async fn event_loop(&mut self) {
        loop {
            match read(&mut self.stream).await {
                None => {}
                Some(result) => match result {
                    Ok(message) => match self.handle_message(message).await {
                        Ok(MessageResult::Continue) => {}
                        Ok(MessageResult::Terminate) => {
                            break;
                        }
                        Err(err) => log::warn!("Could not handle message: {}", err),
                    },
                    Err(err) => log::warn!("Could not read message: {}", err),
                },
            }
        }
    }

    #[doc(hidden)]
    pub(crate) async fn handle_message(
        &mut self,
        message: IPCMessage,
    ) -> Result<MessageResult, String> {
        match message {
            IPCMessage::DeviceSetPropertyCommand(DeviceSetPropertyCommand {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter(&message.adapter_id)?;

                let device = adapter
                    .lock()
                    .await
                    .adapter_handle_mut()
                    .get_device(&message.device_id);

                if let Some(device) = device {
                    let property = device
                        .lock()
                        .await
                        .device_handle_mut()
                        .get_property(&message.property_name)
                        .ok_or_else(|| {
                            format!(
                                "Failed to update property {} of {}: not found",
                                message.property_name, message.device_id,
                            )
                        })?;

                    property
                        .lock()
                        .await
                        .on_update(message.property_value.clone())
                        .await?;

                    property
                        .lock()
                        .await
                        .property_handle_mut()
                        .set_value(message.property_value.clone())
                        .map_err(|err| {
                            format!(
                                "Failed to update property {} of {}: {}",
                                message.property_name, message.device_id, err,
                            )
                        })
                        .await?;
                }

                Ok(MessageResult::Continue)
            }
            IPCMessage::AdapterUnloadRequest(AdapterUnloadRequest {
                message_type: _,
                data: message,
            }) => {
                log::info!(
                    "Received request to unload adapter '{}'",
                    message.adapter_id
                );

                let adapter = self.borrow_adapter(&message.adapter_id)?;

                adapter
                    .lock()
                    .await
                    .adapter_handle_mut()
                    .unload()
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::PluginUnloadRequest(PluginUnloadRequest {
                message_type: _,
                data: message,
            }) => {
                log::info!("Received request to unload plugin '{}'", message.plugin_id);

                self.unload()
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;

                Ok(MessageResult::Terminate)
            }
            IPCMessage::DeviceSavedNotification(DeviceSavedNotification {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter(&message.adapter_id)?;

                adapter
                    .lock()
                    .await
                    .on_device_saved(message.device_id, message.device)
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::AdapterStartPairingCommand(AdapterStartPairingCommand {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter(&message.adapter_id)?;

                adapter
                    .lock()
                    .await
                    .on_start_pairing(Duration::from_secs(message.timeout as u64))
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::AdapterCancelPairingCommand(AdapterCancelPairingCommand {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter(&message.adapter_id)?;

                adapter
                    .lock()
                    .await
                    .on_cancel_pairing()
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::AdapterRemoveDeviceRequest(AdapterRemoveDeviceRequest {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter(&message.adapter_id)?;

                let mut adapter = adapter.lock().await;

                adapter
                    .on_remove_device(message.device_id.clone())
                    .await
                    .map_err(|err| format!("Could not execute remove device callback: {}", err))?;

                adapter
                    .adapter_handle_mut()
                    .remove_device(&message.device_id)
                    .await
                    .map_err(|err| format!("Could not send unload response: {}", err))?;

                Ok(MessageResult::Continue)
            }
            msg => Err(format!("Unexpected msg: {:?}", msg)),
        }
    }

    pub fn borrow_adapter(
        &mut self,
        adapter_id: &str,
    ) -> Result<&mut Arc<Mutex<dyn Adapter>>, String> {
        self.adapters
            .get_mut(adapter_id)
            .ok_or_else(|| format!("Cannot find adapter '{}'", adapter_id))
    }

    pub async fn create_adapter<T, F>(
        &mut self,
        adapter_id: &str,
        name: &str,
        constructor: F,
    ) -> Result<Arc<Mutex<T>>, ApiError>
    where
        T: Adapter + 'static,
        F: FnOnce(AdapterHandle) -> T,
    {
        let message: Message = AdapterAddedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: adapter_id.to_owned(),
            name: name.to_owned(),
            package_name: self.plugin_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        let adapter_handle = AdapterHandle::new(
            self.client.clone(),
            self.plugin_id.clone(),
            adapter_id.to_owned(),
        );

        let adapter = Arc::new(Mutex::new(constructor(adapter_handle)));

        self.adapters.insert(adapter_id.to_owned(), adapter.clone());

        Ok(adapter)
    }

    pub async fn unload(&self) -> Result<(), ApiError> {
        let message: Message = PluginUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }

    pub async fn fail(&self, message: String) -> Result<(), ApiError> {
        let message: Message = PluginErrorNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            message,
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        self.unload().await?;

        sleep(Duration::from_millis(500)).await;

        process::exit(DONT_RESTART_EXIT_CODE);
    }

    pub fn get_config_database(&self) -> Database {
        let config_path = PathBuf::from(self.user_profile.config_dir.clone());
        Database::new(config_path, self.plugin_id.clone())
    }
}
