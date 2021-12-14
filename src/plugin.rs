/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! Connection to the WebthingsIO gateway.

use crate::{
    adapter::{Adapter, AdapterHandle},
    api_error::ApiError,
    client::Client,
    database::Database,
};
use futures::prelude::*;
use mockall_double::double;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, path::PathBuf, process, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use webthings_gateway_ipc_types::{
    AdapterAddedNotificationMessageData, AdapterCancelPairingCommand, AdapterRemoveDeviceRequest,
    AdapterStartPairingCommand, AdapterUnloadRequest, DeviceRemoveActionRequest,
    DeviceRemoveActionResponseMessageData, DeviceRequestActionRequest,
    DeviceRequestActionResponseMessageData, DeviceSavedNotification, DeviceSetPropertyCommand,
    Message, Message as IPCMessage, PluginErrorNotificationMessageData, PluginUnloadRequest,
    PluginUnloadResponseMessageData, Preferences, UserProfile,
};

const DONT_RESTART_EXIT_CODE: i32 = 100;

mod double {
    #[cfg(not(test))]
    pub mod plugin {
        use crate::{api_error::ApiError, client::Client, plugin::Plugin};
        use futures::stream::{SplitStream, StreamExt};
        use std::{collections::HashMap, str::FromStr, sync::Arc};
        use tokio::{net::TcpStream, sync::Mutex};
        use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
        use url::Url;
        use webthings_gateway_ipc_types::{
            Message as IPCMessage, PluginRegisterRequestMessageData,
            PluginRegisterResponseMessageData,
        };

        pub(crate) type PluginStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
        const GATEWAY_URL: &str = "ws://localhost:9500";

        /// Connect to a WebthingsIO gateway and create a new [plugin][Plugin].
        pub async fn connect(plugin_id: impl Into<String>) -> Result<Plugin, ApiError> {
            let plugin_id = plugin_id.into();
            let url = Url::parse(GATEWAY_URL).expect("Could not parse url");

            let (socket, _) = connect_async(url).await.map_err(ApiError::Connect)?;

            let (sink, mut stream) = socket.split();
            let mut client = Client::new(sink);

            let message: IPCMessage = PluginRegisterRequestMessageData {
                plugin_id: plugin_id.clone(),
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
                plugin_id,
                preferences,
                user_profile,
                client: Arc::new(Mutex::new(client)),
                stream,
                adapters: HashMap::new(),
            })
        }

        pub(crate) async fn read(stream: &mut PluginStream) -> Option<Result<IPCMessage, String>> {
            stream.next().await.map(|result| match result {
                Ok(msg) => {
                    let json = msg
                        .to_text()
                        .map_err(|err| format!("Could not get text message: {:?}", err))?;

                    log::trace!("Received message {}", json);

                    IPCMessage::from_str(json)
                        .map_err(|err| format!("Could not parse message: {:?}", err))
                }
                Err(err) => Err(err.to_string()),
            })
        }
    }

    #[cfg(test)]
    pub mod mock_plugin {
        use crate::{client::Client, plugin::Plugin};
        use std::{collections::HashMap, sync::Arc};
        use tokio::sync::Mutex;
        use webthings_gateway_ipc_types::{Message as IPCMessage, Preferences, Units, UserProfile};

        pub(crate) type PluginStream = ();

        pub fn connect(plugin_id: impl Into<String>) -> Plugin {
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
                stream: (),
                adapters: HashMap::new(),
            }
        }

        pub(crate) async fn read(_stream: &mut PluginStream) -> Option<Result<IPCMessage, String>> {
            None
        }
    }
}

#[double]
use double::plugin;

pub use plugin::*;

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
    stream: PluginStream,
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
                let adapter = self.borrow_adapter_(&message.adapter_id)?;

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

                let adapter = self.borrow_adapter_(&message.adapter_id)?;

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
                let adapter = self.borrow_adapter_(&message.adapter_id)?;

                adapter
                    .lock()
                    .await
                    .on_device_saved(message.device_id, message.device)
                    .await
                    .map_err(|err| format!("Error during adapter.on_device_saved: {}", err))?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::AdapterStartPairingCommand(AdapterStartPairingCommand {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter_(&message.adapter_id)?;

                adapter
                    .lock()
                    .await
                    .on_start_pairing(Duration::from_secs(message.timeout as u64))
                    .await
                    .map_err(|err| format!("Error during adapter.on_start_pairing: {}", err))?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::AdapterCancelPairingCommand(AdapterCancelPairingCommand {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter_(&message.adapter_id)?;

                adapter
                    .lock()
                    .await
                    .on_cancel_pairing()
                    .await
                    .map_err(|err| format!("Error during adapter.on_cancel_pairing: {}", err))?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::AdapterRemoveDeviceRequest(AdapterRemoveDeviceRequest {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter_(&message.adapter_id)?;

                let mut adapter = adapter.lock().await;

                adapter
                    .on_remove_device(message.device_id.clone())
                    .await
                    .map_err(|err| format!("Could not execute remove device callback: {}", err))?;

                adapter
                    .adapter_handle_mut()
                    .remove_device(&message.device_id)
                    .await
                    .map_err(|err| {
                        format!("Could not remove device from adapter handle: {}", err)
                    })?;

                Ok(MessageResult::Continue)
            }
            IPCMessage::DeviceRequestActionRequest(DeviceRequestActionRequest {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter_(&message.adapter_id)?;

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
            IPCMessage::DeviceRemoveActionRequest(DeviceRemoveActionRequest {
                message_type: _,
                data: message,
            }) => {
                let adapter = self.borrow_adapter_(&message.adapter_id)?;

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
                        .remove_action(action_name.clone(), action_id.clone())
                        .await
                    {
                        let message = DeviceRemoveActionResponseMessageData {
                            plugin_id: message.plugin_id,
                            adapter_id: message.adapter_id,
                            device_id: device_id.clone(),
                            action_name: action_name.clone(),
                            action_id: action_id.clone(),
                            message_id: message.message_id,
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
                            "Failed to remove action {} ({}) for device {}: {:?}",
                            action_name, action_id, device_id, err
                        ));
                    } else {
                        let message = DeviceRemoveActionResponseMessageData {
                            plugin_id: message.plugin_id,
                            adapter_id: message.adapter_id,
                            device_id: device_id.clone(),
                            action_name: action_name.clone(),
                            action_id: action_id.clone(),
                            message_id: message.message_id,
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

    fn borrow_adapter_(
        &mut self,
        adapter_id: &str,
    ) -> Result<&mut Arc<Mutex<Box<dyn Adapter>>>, String> {
        self.borrow_adapter(adapter_id)
            .map_err(|e| format!("{:?}", e))
    }

    /// Borrow the adapter with the given id.
    pub fn borrow_adapter(
        &mut self,
        adapter_id: impl Into<String>,
    ) -> Result<&mut Arc<Mutex<Box<dyn Adapter>>>, ApiError> {
        let adapter_id = adapter_id.into();
        self.adapters
            .get_mut(&adapter_id)
            .ok_or(ApiError::UnknownAdapter(adapter_id))
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
    pub async fn create_adapter<T, F>(
        &mut self,
        adapter_id: impl Into<String>,
        name: impl Into<String>,
        constructor: F,
    ) -> Result<Arc<Mutex<Box<dyn Adapter>>>, ApiError>
    where
        T: Adapter,
        F: FnOnce(AdapterHandle) -> T,
    {
        let adapter_id = adapter_id.into();

        let message: Message = AdapterAddedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: adapter_id.clone(),
            name: name.into(),
            package_name: self.plugin_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        let adapter_handle = AdapterHandle::new(
            self.client.clone(),
            self.plugin_id.clone(),
            adapter_id.clone(),
        );

        let adapter: Arc<Mutex<Box<dyn Adapter>>> =
            Arc::new(Mutex::new(Box::new(constructor(adapter_handle))));
        let adapter_weak = Arc::downgrade(&adapter);
        adapter.lock().await.adapter_handle_mut().weak = adapter_weak;
        self.adapters.insert(adapter_id, adapter.clone());

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
    pub async fn fail(&self, message: impl Into<String>) -> Result<(), ApiError> {
        let message: Message = PluginErrorNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            message: message.into(),
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
        device::tests::MockDevice,
        event::{EventHandle, NoData},
        plugin::{connect, Plugin},
        property::{self, tests::MockProperty, PropertyHandle},
    };
    use as_any::Downcast;
    use rstest::{fixture, rstest};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{
        AdapterCancelPairingCommandMessageData, AdapterRemoveDeviceRequestMessageData,
        AdapterStartPairingCommandMessageData, AdapterUnloadRequestMessageData,
        DeviceRemoveActionRequestMessageData, DeviceRequestActionRequestMessageData,
        DeviceSavedNotificationMessageData, DeviceSetPropertyCommandMessageData, DeviceWithoutId,
        Message, PluginUnloadRequestMessageData,
    };

    async fn add_mock_adapter(
        plugin: &mut Plugin,
        adapter_id: &str,
    ) -> Arc<Mutex<Box<dyn Adapter>>> {
        let plugin_id = plugin.plugin_id.to_owned();
        let adapter_id_clone = adapter_id.to_owned();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterAddedNotification(msg) => {
                    msg.data.plugin_id == plugin_id && msg.data.adapter_id == adapter_id_clone
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

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";
    const ACTION_ID: &str = "action_id";

    #[fixture]
    fn plugin() -> Plugin {
        connect(PLUGIN_ID)
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_adapter(mut plugin: Plugin) {
        add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        assert!(plugin.borrow_adapter(ADAPTER_ID).is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn test_borrow_unknown_adapter(mut plugin: Plugin) {
        assert!(plugin.borrow_adapter(ADAPTER_ID).is_err());
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_remove_device(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        let message: Message = AdapterRemoveDeviceRequestMessageData {
            device_id: DEVICE_ID.to_owned(),
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterRemoveDeviceResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_remove_device()
                .withf(move |device_id| device_id == DEVICE_ID)
                .times(1)
                .returning(|_| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();

        assert!(adapter
            .lock()
            .await
            .adapter_handle_mut()
            .get_device(DEVICE_ID)
            .is_none())
    }

    #[rstest]
    #[case(MockDevice::ACTION_NOINPUT, json!(null), NoInput)]
    #[case(MockDevice::ACTION_BOOL, json!(true), true)]
    #[case(MockDevice::ACTION_U8, json!(112_u8), 112_u8)]
    #[case(MockDevice::ACTION_I32, json!(21), 21)]
    #[case(MockDevice::ACTION_F32, json!(-2.7_f32), -2.7_f32)]
    #[case(MockDevice::ACTION_OPTI32, json!(11), Some(11))]
    #[case(MockDevice::ACTION_OPTI32, json!(null), Option::<i32>::None)]
    #[case(MockDevice::ACTION_STRING, json!("foo"), "foo".to_owned())]
    #[tokio::test]
    async fn test_request_action_perform<T: Input + PartialEq>(
        #[case] action_name: &'static str,
        #[case] action_input: serde_json::Value,
        #[case] expected_input: T,
        mut plugin: Plugin,
    ) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        {
            let mut device = device.lock().await;
            let action = device.device_handle_mut().get_action(action_name).unwrap();
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
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            action_name: action_name.to_owned(),
            action_id: ACTION_ID.to_owned(),
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
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.action_name == action_name
                        && msg.data.action_id == ACTION_ID
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_action_cancel(mut plugin: Plugin) {
        let message_id = 42;
        let action_name = MockDevice::ACTION_I32.to_owned();
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        {
            let mut device = device.lock().await;
            let action = device
                .device_handle_mut()
                .get_action(action_name.to_owned())
                .unwrap();
            let mut action = action.lock().await;
            let action = action
                .as_any_mut()
                .downcast_mut::<MockAction<i32>>()
                .unwrap();
            action
                .action_helper
                .expect_cancel()
                .withf(move |action_id| action_id == ACTION_ID)
                .times(1)
                .returning(|_| Ok(()));
        }

        let message: Message = DeviceRemoveActionRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            action_name: action_name.to_owned(),
            action_id: ACTION_ID.to_owned(),
            message_id,
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceRemoveActionResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.action_name == action_name
                        && msg.data.action_id == ACTION_ID
                        && msg.data.message_id == message_id
                        && msg.data.success == true
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[case(MockDevice::PROPERTY_BOOL, json!(true), true)]
    #[case(MockDevice::PROPERTY_U8, json!(112_u8), 112_u8)]
    #[case(MockDevice::PROPERTY_I32, json!(21), 21)]
    #[case(MockDevice::PROPERTY_F32, json!(-2.7_f32), -2.7_f32)]
    #[case(MockDevice::PROPERTY_OPTI32, json!(11), Some(11))]
    #[case(MockDevice::PROPERTY_OPTI32, json!(null), Option::<i32>::None)]
    #[case(MockDevice::PROPERTY_STRING, json!("foo"), "foo".to_owned())]
    #[tokio::test]
    async fn test_request_property_update_value<T: property::Value + PartialEq>(
        #[case] property_name: &'static str,
        #[case] property_value: serde_json::Value,
        #[case] expected_value: T,
        mut plugin: Plugin,
    ) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        {
            let expected_value = expected_value.clone();
            let mut device = device.lock().await;
            let property = device
                .device_handle_mut()
                .get_property(property_name)
                .unwrap();
            let mut property = property.lock().await;
            let property = property.downcast_mut::<MockProperty<T>>().unwrap();
            property
                .property_helper
                .expect_on_update()
                .withf(move |value| value == &expected_value)
                .times(1)
                .returning(|_| Ok(()));
        }

        let serialized_value = property::Value::serialize(expected_value.clone()).unwrap();

        let message: Message = DeviceSetPropertyCommandMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            property_name: property_name.to_owned(),
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
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.property.name == Some(property_name.to_owned())
                        && msg.data.property.value == serialized_value
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

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

    #[rstest]
    #[tokio::test]
    async fn test_request_adapter_unload(mut plugin: Plugin) {
        add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = AdapterUnloadRequestMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
        }
        .into();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterUnloadResponse(msg) => {
                    msg.data.plugin_id == PLUGIN_ID && msg.data.adapter_id == ADAPTER_ID
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_adapter_start_pairing(mut plugin: Plugin) {
        let timeout = 5000;
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = AdapterStartPairingCommandMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            timeout: timeout,
        }
        .into();

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_start_pairing()
                .withf(move |t| t.as_secs() == timeout as u64)
                .times(1)
                .returning(|_| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_request_adapter_cancel_pairing(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = AdapterCancelPairingCommandMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
        }
        .into();

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_cancel_pairing()
                .times(1)
                .returning(|| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_notification_device_saved(mut plugin: Plugin) {
        let device_description = DeviceWithoutId {
            at_context: None,
            at_type: None,
            actions: None,
            base_href: None,
            credentials_required: None,
            description: None,
            events: None,
            links: None,
            pin: None,
            properties: None,
            title: None,
        };
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;

        let message: Message = DeviceSavedNotificationMessageData {
            plugin_id: PLUGIN_ID.to_owned(),
            adapter_id: ADAPTER_ID.to_owned(),
            device_id: DEVICE_ID.to_owned(),
            device: device_description.clone(),
        }
        .into();

        {
            let mut adapter = adapter.lock().await;
            let adapter = adapter.downcast_mut::<MockAdapter>().unwrap();
            adapter
                .adapter_helper
                .expect_on_device_saved()
                .withf(move |id, description| id == DEVICE_ID && description == &device_description)
                .times(1)
                .returning(|_, _| Ok(()));
        }

        plugin.handle_message(message).await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_config_database(plugin: Plugin) {
        let db = plugin.get_config_database::<serde_json::Value>();
        assert_eq!(db.plugin_id, PLUGIN_ID);
    }

    #[rstest]
    #[tokio::test]
    async fn test_device_has_weak_adapter_ref(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        assert!(device
            .lock()
            .await
            .device_handle_mut()
            .adapter
            .upgrade()
            .is_some())
    }

    #[rstest]
    #[tokio::test]
    async fn test_property_has_weak_device_ref(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        let property = device
            .lock()
            .await
            .device_handle_mut()
            .get_property(MockDevice::PROPERTY_I32)
            .unwrap();
        assert!(property
            .lock()
            .await
            .property_handle_mut()
            .downcast_ref::<PropertyHandle<i32>>()
            .unwrap()
            .device
            .upgrade()
            .is_some())
    }

    #[rstest]
    #[tokio::test]
    async fn test_event_has_weak_device_ref(mut plugin: Plugin) {
        let adapter = add_mock_adapter(&mut plugin, ADAPTER_ID).await;
        let device = add_mock_device(adapter.lock().await.adapter_handle_mut(), DEVICE_ID).await;

        let event = device
            .lock()
            .await
            .device_handle_mut()
            .get_event(MockDevice::EVENT_NODATA)
            .unwrap();
        assert!(event
            .lock()
            .await
            .downcast_ref::<EventHandle<NoData>>()
            .unwrap()
            .device
            .upgrade()
            .is_some())
    }
}
