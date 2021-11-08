/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! Connection to the WebthingsIO gateway.

use crate::{
    adapter::{Adapter, AdapterHandle},
    api_error::ApiError,
    client::{Client, ClientExt},
    database::Database,
};
use futures::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, path::PathBuf, process, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use webthings_gateway_ipc_types::{
    AdapterAddedNotificationMessageData, AdapterCancelPairingCommand, AdapterRemoveDeviceRequest,
    AdapterStartPairingCommand, AdapterUnloadRequest, DeviceRequestActionRequest,
    DeviceRequestActionResponseMessageData, DeviceSavedNotification, DeviceSetPropertyCommand,
    Message, Message as IPCMessage, PluginErrorNotificationMessageData, PluginUnloadRequest,
    PluginUnloadResponseMessageData, Preferences, UserProfile,
};

#[cfg(not(test))]
use {
    futures::stream::SplitStream,
    std::str::FromStr,
    tokio::net::TcpStream,
    tokio_tungstenite::connect_async,
    tokio_tungstenite::{MaybeTlsStream, WebSocketStream},
    url::Url,
    webthings_gateway_ipc_types::{
        PluginRegisterRequestMessageData, PluginRegisterResponseMessageData,
    },
};

#[cfg(test)]
use webthings_gateway_ipc_types::Units;

#[cfg(not(test))]
const GATEWAY_URL: &str = "ws://localhost:9500";
const DONT_RESTART_EXIT_CODE: i32 = 100;

/// Connect to a WebthingsIO gateway and create a new [plugin][Plugin].
#[cfg(not(test))]
pub async fn connect(plugin_id: &str) -> Result<Plugin, ApiError> {
    let url = Url::parse(GATEWAY_URL).expect("Could not parse url");

    let (socket, _) = connect_async(url).await.map_err(ApiError::Connect)?;

    let (sink, mut stream) = socket.split();
    let mut client = Client::new(sink);

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
pub fn connect<S: Into<String>>(plugin_id: S) -> Plugin {
    let preferences = Preferences {
        language: "en-US".to_owned(),
        units: Units {
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
    let client = Arc::new(Mutex::new(Client::new()));
    Plugin {
        plugin_id: plugin_id.into(),
        preferences,
        user_profile,
        client: client.clone(),
        adapters: HashMap::new(),
    }
}

#[cfg(not(test))]
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
    pub(crate) client: Arc<Mutex<Client>>,
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

#[cfg(test)]
mod tests {
    use crate::{
        action::{tests::MockAction, Input, NoInput},
        adapter::{
            tests::{add_mock_device, MockAdapter},
            Adapter,
        },
        plugin::{connect, Plugin},
        property::{self, tests::MockProperty},
    };
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{
        AdapterRemoveDeviceRequestMessageData, DeviceRequestActionRequestMessageData,
        DeviceSetPropertyCommandMessageData, Message,
    };

    async fn add_mock_adapter(
        plugin: &mut Plugin,
        adapter_id: &str,
    ) -> Arc<Mutex<Box<dyn Adapter>>> {
        let plugin_id = plugin.plugin_id.to_owned();
        let adapter_id_copy = adapter_id.to_owned();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterAddedNotification(msg) => {
                    msg.data.plugin_id == plugin_id && msg.data.adapter_id == adapter_id_copy
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin
            .create_adapter(adapter_id, adapter_id, |adapter| MockAdapter::new(adapter))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_create_adapter() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let mut plugin = connect(&plugin_id);
        add_mock_adapter(&mut plugin, &adapter_id).await;
        assert!(plugin.borrow_adapter(&adapter_id).is_ok());
    }

    #[tokio::test]
    async fn test_request_remove_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let device_id_copy = device_id.clone();
        let mut plugin = connect(&plugin_id);
        let adapter = add_mock_adapter(&mut plugin, &adapter_id).await;
        add_mock_device(adapter.lock().await.adapter_handle_mut(), &device_id).await;

        let message: Message = AdapterRemoveDeviceRequestMessageData {
            device_id: device_id.clone(),
            plugin_id: plugin_id.clone(),
            adapter_id: adapter_id.to_owned(),
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterRemoveDeviceResponse(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();

        assert!(adapter
            .lock()
            .await
            .adapter_handle_mut()
            .get_device(&device_id_copy)
            .is_none())
    }

    async fn test_request_action_perform<T: Input + PartialEq>(
        action_name: String,
        action_input: serde_json::Value,
        expected_input: T,
    ) {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let action_id = String::from("action_id");

        let mut plugin = connect(&plugin_id);
        let adapter = add_mock_adapter(&mut plugin, &adapter_id).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), &device_id).await;

        {
            let mut device = device.lock().await;
            let action = device.device_handle_mut().get_action(&action_name).unwrap();
            let mut action = action.lock().await;
            let action = action.as_any_mut().downcast_mut::<MockAction<T>>().unwrap();
            action
                .action_helper
                .expect_perform()
                .withf(move |action_handle| action_handle.input == expected_input)
                .times(1)
                .returning(|_| Ok(()));
        }

        let message: Message = DeviceRequestActionRequestMessageData {
            plugin_id: plugin_id.clone(),
            adapter_id: adapter_id.clone(),
            device_id: device_id.clone(),
            action_name: action_name.clone(),
            action_id: action_id.clone(),
            input: action_input,
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceRequestActionResponse(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                        && msg.data.action_name == action_name
                        && msg.data.action_id == action_id
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[tokio::test]
    async fn test_request_action_noinput_perform() {
        test_request_action_perform(String::from("action_noinput"), json!(null), NoInput).await;
    }

    #[tokio::test]
    async fn test_request_action_bool_perform() {
        test_request_action_perform(String::from("action_bool"), json!(true), true).await;
    }

    #[tokio::test]
    async fn test_request_action_u8_perform() {
        test_request_action_perform(String::from("action_u8"), json!(112_u8), 112_u8).await;
    }

    #[tokio::test]
    async fn test_request_action_i32_perform() {
        test_request_action_perform(String::from("action_i32"), json!(21), 21).await;
    }

    #[tokio::test]
    async fn test_request_action_f32_perform() {
        test_request_action_perform(String::from("action_f32"), json!(-2.7_f32), -2.7_f32).await;
    }

    #[tokio::test]
    async fn test_request_action_opti32_perform() {
        test_request_action_perform(String::from("action_opti32"), json!(11), Some(11)).await;
        test_request_action_perform::<Option<i32>>(
            String::from("action_opti32"),
            json!(null),
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn test_request_action_string_perform() {
        test_request_action_perform(
            String::from("action_string"),
            json!("foo"),
            "foo".to_owned(),
        )
        .await;
    }

    async fn test_request_property_update_value<T: property::Value + PartialEq>(
        property_name: String,
        property_value: serde_json::Value,
        expected_value: T,
    ) {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");

        let mut plugin = connect(&plugin_id);
        let adapter = add_mock_adapter(&mut plugin, &adapter_id).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), &device_id).await;

        {
            let expected_value = expected_value.clone();
            let mut device = device.lock().await;
            let property = device
                .device_handle_mut()
                .get_property(&property_name)
                .unwrap();
            let mut property = property.lock().await;
            let property = property
                .as_any_mut()
                .downcast_mut::<MockProperty<T>>()
                .unwrap();
            property
                .property_helper
                .expect_on_update()
                .withf(move |value| value == &expected_value)
                .times(1)
                .returning(|_| Ok(()));
        }

        let serialized_value = property::Value::serialize(expected_value.clone()).unwrap();

        let message: Message = DeviceSetPropertyCommandMessageData {
            plugin_id: plugin_id.clone(),
            adapter_id: adapter_id.clone(),
            device_id: device_id.clone(),
            property_name: property_name.clone(),
            property_value,
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DevicePropertyChangedNotification(msg) => {
                    println!("asdf {:?} {:?}", msg.data.property.value, serialized_value);
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                        && msg.data.property.name == Some(property_name.to_owned())
                        && msg.data.property.value == serialized_value
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[tokio::test]
    async fn test_request_property_bool_update_value() {
        test_request_property_update_value(String::from("property_bool"), json!(true), true).await;
    }

    #[tokio::test]
    async fn test_request_property_u8_update_value() {
        test_request_property_update_value(String::from("property_u8"), json!(112_u8), 112_u8)
            .await;
    }

    #[tokio::test]
    async fn test_request_property_i32_update_value() {
        test_request_property_update_value(String::from("property_i32"), json!(21), 21).await;
    }

    #[tokio::test]
    async fn test_request_property_f32_update_value() {
        test_request_property_update_value(String::from("property_f32"), json!(-2.7_f32), -2.7_f32)
            .await;
    }

    #[tokio::test]
    async fn test_request_property_opti32_update_value() {
        test_request_property_update_value(String::from("property_opti32"), json!(21), Some(21))
            .await;
        test_request_property_update_value::<Option<i32>>(
            String::from("property_opti32"),
            json!(null),
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn test_request_property_string_update_value() {
        test_request_property_update_value(
            String::from("property_string"),
            json!("foo"),
            "foo".to_owned(),
        )
        .await;
    }
}
