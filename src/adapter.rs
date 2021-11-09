/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WebthingsIO adapters.

use crate::{
    api_error::ApiError,
    client::Client,
    device::{self, Device, DeviceBuilder},
};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    AdapterRemoveDeviceResponseMessageData, AdapterUnloadResponseMessageData,
    DeviceAddedNotificationMessageData, DeviceWithoutId, Message,
};

/// A trait used to specify the behaviour of a WebthingsIO adapter.
///
/// Wraps an [adapter handle][AdapterHandle] and defines how to react on gateway requests. Created through a [plugin][crate::plugin::Plugin].
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{prelude::*, plugin::connect, example::ExampleDeviceBuilder};
/// # use webthings_gateway_ipc_types::DeviceWithoutId;
/// # use async_trait::async_trait;
/// struct ExampleAdapter(AdapterHandle);
///
/// #[async_trait]
/// impl Adapter for ExampleAdapter {
///     fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
///         &mut self.0
///     }
///
///     async fn on_remove_device(&mut self, device_id: String) -> Result<(), String> {
///         log::debug!("Device {} removed", device_id);
///         Ok(())
///     }
/// }
///
/// impl ExampleAdapter {
/// #   pub fn new(adapter_handle: AdapterHandle) -> Self {
/// #       Self(adapter_handle)
/// #   }
///
///     pub async fn init(&mut self) -> Result<(), ApiError> {
///         self.adapter_handle_mut()
///             .add_device(ExampleDeviceBuilder::new())
///             .await?;
///         Ok(())
///     }
/// }
///
/// # #[tokio::main]
/// pub async fn main() -> Result<(), ApiError> {
///     let mut plugin = connect("example-addon").await?;
///     let adapter = plugin
///         .create_adapter("example-adapter", "Example Adapter", ExampleAdapter::new)
///         .await?;
///     adapter
///         .lock()
///         .await
///         .as_any_mut()
///         .downcast_mut::<ExampleAdapter>()
///         .unwrap()
///         .init()
///         .await?;
///     plugin.event_loop().await;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait Adapter: Send + Sync + AsAny + 'static {
    /// Return the wrapped [adapter handle][AdapterHandle].
    fn adapter_handle_mut(&mut self) -> &mut AdapterHandle;

    /// Called when a new [device][crate::device::Device] was saved within the gateway.
    ///
    /// This happens when a thing was added through the add things view.
    async fn on_device_saved(
        &mut self,
        _device_id: String,
        _device_description: DeviceWithoutId,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Called when the gateway starts pairing.
    ///
    /// This happens when the add things view opens.
    async fn on_start_pairing(&mut self, _timeout: Duration) -> Result<(), String> {
        Ok(())
    }

    /// Called when the gateway stops pairing.
    ///
    /// This happens when the add things view closes.
    async fn on_cancel_pairing(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Called when a previously saved [device][crate::device::Device] was removed.
    ///
    /// This happens when an added thing was removed through the gateway.
    async fn on_remove_device(&mut self, _device_id: String) -> Result<(), String> {
        Ok(())
    }
}

impl Downcast for dyn Adapter {}

/// A struct which represents an instance of a WebthingsIO adapter.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct AdapterHandle {
    client: Arc<Mutex<Client>>,
    pub(crate) weak: Weak<Mutex<Box<dyn Adapter>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    devices: HashMap<String, Arc<Mutex<Box<dyn Device>>>>,
}

impl AdapterHandle {
    pub(crate) fn new(client: Arc<Mutex<Client>>, plugin_id: String, adapter_id: String) -> Self {
        Self {
            client,
            weak: Weak::new(),
            plugin_id,
            adapter_id,
            devices: HashMap::new(),
        }
    }

    /// Build and add a new device using the given [device builder][crate::device::DeviceBuilder].
    pub async fn add_device<D, B>(
        &mut self,
        device_builder: B,
    ) -> Result<Arc<Mutex<Box<dyn Device>>>, ApiError>
    where
        D: Device,
        B: DeviceBuilder<Device = D>,
    {
        let device_description = device_builder.full_description()?;

        let message: Message = DeviceAddedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device: device_description.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        let id = device_description.id.clone();

        let device_handle = device::DeviceHandle::new(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            device_builder.id(),
            device_builder.description(),
        );

        let properties = device_builder.properties();
        let actions = device_builder.actions();
        let events = device_builder.events();

        let device: Arc<Mutex<Box<dyn Device>>> =
            Arc::new(Mutex::new(Box::new(device_builder.build(device_handle))));
        let device_weak = Arc::downgrade(&device);

        {
            let mut device = device.lock().await;
            let device_handle = device.device_handle_mut();
            device_handle.weak = device_weak;

            for property_builder in properties {
                device_handle.add_property(property_builder);
            }

            for action in actions {
                device_handle.add_action(action);
            }

            for event in events {
                device_handle.add_event(event);
            }
        }

        self.devices.insert(id, device.clone());

        Ok(device)
    }

    /// Get a reference to all the [devices][crate::device::Device] which this adapter owns.
    pub fn devices(&self) -> &HashMap<String, Arc<Mutex<Box<dyn Device>>>> {
        &self.devices
    }

    /// Get a [device][crate::device::Device] which this adapter owns by ID.
    pub fn get_device(&self, id: &str) -> Option<Arc<Mutex<Box<dyn Device>>>> {
        self.devices.get(id).cloned()
    }

    /// Unload this adapter.
    pub async fn unload(&self) -> Result<(), ApiError> {
        let message: Message = AdapterUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }

    /// Remove a [device][crate::device::Device] which this adapter owns by ID.
    pub async fn remove_device<S: Into<String> + Clone>(
        &mut self,
        device_id: S,
    ) -> Result<(), String> {
        if self.devices.remove(&device_id.clone().into()).is_none() {
            return Err("Unknown device".to_owned());
        }

        let message: Message = AdapterRemoveDeviceResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: device_id.into(),
        }
        .into();

        self.client
            .lock()
            .await
            .send_message(&message)
            .await
            .map_err(|err| format!("Could not send response: {}", err))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        adapter::{Adapter, AdapterHandle},
        client::Client,
        device::{tests::MockDeviceBuilder, Device, DeviceBuilder},
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    pub struct MockAdapter {
        adapter_handle: AdapterHandle,
    }

    impl MockAdapter {
        pub fn new(adapter_handle: AdapterHandle) -> Self {
            Self { adapter_handle }
        }
    }

    impl Adapter for MockAdapter {
        fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
            &mut self.adapter_handle
        }
    }

    pub async fn add_mock_device(
        adapter: &mut AdapterHandle,
        device_id: &str,
    ) -> Arc<Mutex<Box<dyn Device>>> {
        let device_builder = MockDeviceBuilder::new(device_id.to_owned());
        let expected_description = device_builder.full_description().unwrap();

        let plugin_id = adapter.plugin_id.to_owned();
        let adapter_id = adapter.adapter_id.to_owned();

        adapter
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceAddedNotification(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device == expected_description
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        adapter.add_device(device_builder).await.unwrap()
    }

    #[tokio::test]
    async fn test_add_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let client = Arc::new(Mutex::new(Client::new()));

        let mut adapter = AdapterHandle::new(client.clone(), plugin_id, adapter_id);

        add_mock_device(&mut adapter, &device_id).await;

        assert!(adapter.get_device(&device_id).is_some())
    }

    #[tokio::test]
    async fn test_get_unknown_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let client = Arc::new(Mutex::new(Client::new()));

        let adapter = AdapterHandle::new(client.clone(), plugin_id, adapter_id);

        assert!(adapter.get_device(&device_id).is_none())
    }

    #[tokio::test]
    async fn test_remove_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let device_id_copy = device_id.clone();
        let client = Arc::new(Mutex::new(Client::new()));

        let mut adapter = AdapterHandle::new(client.clone(), plugin_id.clone(), adapter_id.clone());

        add_mock_device(&mut adapter, &device_id).await;

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterRemoveDeviceResponse(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id_copy
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        adapter.remove_device(device_id.clone()).await.unwrap();

        assert!(adapter.get_device(&device_id).is_none())
    }

    #[tokio::test]
    async fn test_remove_unknown_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let client = Arc::new(Mutex::new(Client::new()));

        let mut adapter = AdapterHandle::new(client.clone(), plugin_id.clone(), adapter_id.clone());

        assert!(adapter.remove_device(device_id).await.is_err())
    }

    #[tokio::test]
    async fn test_unload() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let client = Arc::new(Mutex::new(Client::new()));

        let adapter = AdapterHandle::new(client.clone(), plugin_id.clone(), adapter_id.clone());

        adapter
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::AdapterUnloadResponse(msg) => {
                    msg.data.plugin_id == plugin_id && msg.data.adapter_id == adapter_id
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        adapter.unload().await.unwrap();
    }
}
