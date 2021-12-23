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
    plugin::{plugin_connection, PluginStream},
    Adapter, AdapterHandle,
};
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

const DONT_RESTART_EXIT_CODE: i32 = 100;

/// A struct which represents a successfully established connection to a WebthingsIO gateway.
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{plugin::connect, error::WebthingsError};
/// #[tokio::main]
/// async fn main() -> Result<(), WebthingsError> {
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
    pub(crate) api_handler: Arc<Mutex<dyn ApiHandler>>,
    pub(crate) stream: PluginStream,
    pub(crate) adapters: HashMap<String, Arc<Mutex<Box<dyn Adapter>>>>,
}

impl Plugin {
    /// Start the event loop of this plugin.
    ///
    /// This will block your current thread.
    pub async fn event_loop(&mut self) {
        loop {
            match plugin_connection::read(&mut self.stream).await {
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

    /// Borrow the adapter with the given id.
    pub fn borrow_adapter(
        &mut self,
        adapter_id: impl Into<String>,
    ) -> Result<&mut Arc<Mutex<Box<dyn Adapter>>>, WebthingsError> {
        let adapter_id = adapter_id.into();
        self.adapters
            .get_mut(&adapter_id)
            .ok_or(WebthingsError::UnknownAdapter(adapter_id))
    }

    /// Create a new adapter.
    ///
    /// # Examples
    /// ```no_run
    /// # use gateway_addon_rust::{prelude::*, plugin::connect, example::ExampleAdapter, error::WebthingsError};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), WebthingsError> {
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
    ) -> Result<Arc<Mutex<Box<dyn Adapter>>>, WebthingsError>
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

    /// Set a new active [ApiHandler](crate::api_handler::ApiHandler).
    pub async fn set_api_handler<T: ApiHandler>(
        &mut self,
        api_handler: T,
    ) -> Result<(), WebthingsError> {
        self.api_handler = Arc::new(Mutex::new(api_handler));
        let message: Message = ApiHandlerAddedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            package_name: self.plugin_id.clone(),
        }
        .into();
        self.client.lock().await.send_message(&message).await?;
        Ok(())
    }

    /// Unload this plugin.
    pub async fn unload(&self) -> Result<(), WebthingsError> {
        let message: Message = PluginUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }

    /// Fail this plugin.
    ///
    /// This should be done when an error occurs which we cannot recover from.
    pub async fn fail(&self, message: impl Into<String>) -> Result<(), WebthingsError> {
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
pub(crate) mod tests {
    use crate::{
        adapter::tests::MockAdapter, api_handler::tests::MockApiHandler, plugin::connect, Adapter,
        Plugin,
    };
    use rstest::{fixture, rstest};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{Message, PluginUnloadRequestMessageData};

    pub async fn add_mock_adapter(
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
            .create_adapter(adapter_id, adapter_id, MockAdapter::new)
            .await
            .unwrap()
    }

    pub async fn set_mock_api_handler(plugin: &mut Plugin) {
        let plugin_id = plugin.plugin_id.to_owned();

        plugin
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::ApiHandlerAddedNotification(msg) => msg.data.plugin_id == plugin_id,
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        plugin.set_api_handler(MockApiHandler::new()).await.unwrap()
    }

    #[fixture]
    pub(crate) fn plugin() -> Plugin {
        connect(PLUGIN_ID)
    }

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";

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
    async fn test_get_config_database(plugin: Plugin) {
        let db = plugin.get_config_database::<serde_json::Value>();
        assert_eq!(db.plugin_id, PLUGIN_ID);
    }
}
