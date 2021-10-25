/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! Connection to the WebthingsIO gateway.

use crate::{
    adapter::{Adapter, AdapterHandle},
    api_error::ApiError,
    client::{Client, WebsocketClient},
    database::Database,
};
use futures::{prelude::*, stream::SplitStream};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, path::PathBuf, process, str::FromStr, sync::Arc, time::Duration};
use tokio::{net::TcpStream, sync::Mutex, time::sleep};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;
use webthings_gateway_ipc_types::{
    AdapterAddedNotificationMessageData, AdapterCancelPairingCommand, AdapterRemoveDeviceRequest,
    AdapterStartPairingCommand, AdapterUnloadRequest, DeviceRequestActionRequest,
    DeviceRequestActionResponseMessageData, DeviceSavedNotification, DeviceSetPropertyCommand,
    Message, Message as IPCMessage, PluginErrorNotificationMessageData,
    PluginRegisterRequestMessageData, PluginRegisterResponseMessageData, PluginUnloadRequest,
    PluginUnloadResponseMessageData, Preferences, UserProfile,
};

const GATEWAY_URL: &str = "ws://localhost:9500";
const DONT_RESTART_EXIT_CODE: i32 = 100;

/// Connect to a WebthingsIO gateway and create a new [plugin][Plugin].
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
pub fn connect<S: Into<String>>(plugin_id: S) -> (Plugin, Arc<Mutex<crate::client::MockClient>>) {
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
            plugin_id: plugin_id.into(),
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

/// A struct which represents a successfully established connection to a WebthingsIO gateway.
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{plugin::connect, api_error::ApiError};
/// #[tokio::main]
/// async fn main() -> Result<(), ApiError> {
///     let mut plugin = connect("example-addon").await?;
///     // ...
///     plugin.event_loop().await;
///     Ok(())
/// }
/// ```
pub struct Plugin {
    pub plugin_id: String,
    pub preferences: Preferences,
    pub user_profile: UserProfile,
    client: Arc<Mutex<dyn Client>>,
    #[cfg(not(test))]
    stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    adapters: HashMap<String, Arc<Mutex<Box<dyn Adapter>>>>,
}

#[doc(hidden)]
pub(crate) enum MessageResult {
    Continue,
    Terminate,
}

impl Plugin {
    /// Start the event loop of this plugin.
    ///
    /// This will block your current thread.
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
                        .set_value(Some(message.property_value.clone()))
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
            IPCMessage::DeviceRequestActionRequest(DeviceRequestActionRequest {
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
                    let mut device = device.lock().await;
                    let device_id = message.device_id;
                    let action_name = message.action_name;
                    let action_id = message.action_id;
                    if let Err(err) = device
                        .device_handle_mut()
                        .request_action(action_name.clone(), action_id.clone(), message.input)
                        .await
                    {
                        let message = DeviceRequestActionResponseMessageData {
                            plugin_id: message.plugin_id,
                            adapter_id: message.adapter_id,
                            device_id: device_id.clone(),
                            action_name: action_name.clone(),
                            action_id: action_id.clone(),
                            success: false,
                        }
                        .into();

                        self.client
                            .lock()
                            .await
                            .send_message(&message)
                            .await
                            .map_err(|err| format!("{:?}", err))?;

                        return Err(format!(
                            "Failed to request action {} for device {}: {:?}",
                            action_name, device_id, err
                        ));
                    } else {
                        let message = DeviceRequestActionResponseMessageData {
                            plugin_id: message.plugin_id,
                            adapter_id: message.adapter_id,
                            device_id: device_id.clone(),
                            action_name: action_name.clone(),
                            action_id: action_id.clone(),
                            success: true,
                        }
                        .into();

                        self.client
                            .lock()
                            .await
                            .send_message(&message)
                            .await
                            .map_err(|err| format!("{:?}", err))?;
                    }
                }

                Ok(MessageResult::Continue)
            }
            msg => Err(format!("Unexpected msg: {:?}", msg)),
        }
    }

    /// Borrow the adapter with the given id.
    pub fn borrow_adapter(
        &mut self,
        adapter_id: &str,
    ) -> Result<&mut Arc<Mutex<Box<dyn Adapter>>>, String> {
        self.adapters
            .get_mut(adapter_id)
            .ok_or_else(|| format!("Cannot find adapter '{}'", adapter_id))
    }

    /// Create a new adapter.
    ///
    /// # Examples
    /// ```no_run
    /// # use gateway_addon_rust::{prelude::*, plugin::connect, example::ExampleAdapter};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), ApiError> {
    /// #   let mut plugin = connect("example-addon").await?;
    /// let adapter = plugin
    ///     .create_adapter("example_adapter", "Example Adapter", |adapter_handle| {
    ///         ExampleAdapter::new(adapter_handle)
    ///     })
    ///     .await?;
    /// #   plugin.event_loop().await;
    /// #   Ok(())
    /// # }
    /// ```
    pub async fn create_adapter<T, F, S>(
        &mut self,
        adapter_id: S,
        name: S,
        constructor: F,
    ) -> Result<Arc<Mutex<Box<dyn Adapter>>>, ApiError>
    where
        T: Adapter,
        F: FnOnce(AdapterHandle) -> T,
        S: Into<String> + Clone,
    {
        let message: Message = AdapterAddedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: adapter_id.clone().into(),
            name: name.into(),
            package_name: self.plugin_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        let adapter_handle = AdapterHandle::new(
            self.client.clone(),
            self.plugin_id.clone(),
            adapter_id.clone().into(),
        );

        let adapter: Arc<Mutex<Box<dyn Adapter>>> =
            Arc::new(Mutex::new(Box::new(constructor(adapter_handle))));
        let adapter_weak = Arc::downgrade(&adapter);
        adapter.lock().await.adapter_handle_mut().weak = adapter_weak;
        self.adapters.insert(adapter_id.into(), adapter.clone());

        Ok(adapter)
    }

    /// Unload this plugin.
    pub async fn unload(&self) -> Result<(), ApiError> {
        let message: Message = PluginUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }

    /// Fail this plugin.
    ///
    /// This should be done when an error occurs which we cannot recover from.
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

    /// Get the associated config database of this plugin.
    pub fn get_config_database<T: Serialize + DeserializeOwned>(&self) -> Database<T> {
        let config_path = PathBuf::from(self.user_profile.config_dir.clone());
        Database::new(config_path, self.plugin_id.clone())
    }
}
