/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WebthingsIO adapters.

use crate::{client::Client, device, error::WebthingsError, Device, DeviceBuilder};
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
/// Wraps an [adapter handle][AdapterHandle] and defines how to react on gateway requests. Created through a [plugin][crate::Plugin].
///
/// # Examples
/// ```no_run
/// # use gateway_addon_rust::{prelude::*, plugin::connect, example::ExampleDeviceBuilder, error::WebthingsError};
/// # use webthings_gateway_ipc_types::DeviceWithoutId;
/// # use async_trait::async_trait;
/// # use as_any::Downcast;
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
///     pub async fn init(&mut self) -> Result<(), WebthingsError> {
///         self.adapter_handle_mut()
///             .add_device(ExampleDeviceBuilder::new())
///             .await?;
///         Ok(())
///     }
/// }
///
/// # #[tokio::main]
/// pub async fn main() -> Result<(), WebthingsError> {
///     let mut plugin = connect("example-addon").await?;
///     let adapter = plugin
///         .create_adapter("example-adapter", "Example Adapter", ExampleAdapter::new)
///         .await?;
///     adapter
///         .lock()
///         .await
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

    /// Called when a new [device][crate::Device] was saved within the gateway.
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

    /// Called when a previously saved [device][crate::Device] was removed.
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

    /// Build and add a new device using the given [device builder][crate::DeviceBuilder].
    pub async fn add_device<D, B>(
        &mut self,
        device_builder: B,
    ) -> Result<Arc<Mutex<Box<dyn Device>>>, WebthingsError>
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

    /// Get a reference to all the [devices][crate::Device] which this adapter owns.
    pub fn devices(&self) -> &HashMap<String, Arc<Mutex<Box<dyn Device>>>> {
        &self.devices
    }

    /// Get a [device][crate::Device] which this adapter owns by ID.
    pub fn get_device(&self, id: impl Into<String>) -> Option<Arc<Mutex<Box<dyn Device>>>> {
        self.devices.get(&id.into()).cloned()
    }

    /// Unload this adapter.
    pub async fn unload(&self) -> Result<(), WebthingsError> {
        let message: Message = AdapterUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }

    /// Remove a [device][crate::Device] which this adapter owns by ID.
    pub async fn remove_device(
        &mut self,
        device_id: impl Into<String>,
    ) -> Result<(), WebthingsError> {
        let device_id = device_id.into();
        if self.devices.remove(&device_id).is_none() {
            return Err(WebthingsError::UnknownDevice(device_id.clone()));
        }

        let message: Message = AdapterRemoveDeviceResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id,
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        client::Client, device::tests::MockDeviceBuilder, Adapter, AdapterHandle, Device,
        DeviceBuilder,
    };
    use async_trait::async_trait;
    use mockall::mock;
    use rstest::{fixture, rstest};
    use std::{sync::Arc, time::Duration};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{DeviceWithoutId, Message};

    mock! {
        pub AdapterHelper {
            pub async fn on_start_pairing(&mut self, timeout: Duration) -> Result<(), String>;
            pub async fn on_cancel_pairing(&mut self) -> Result<(), String>;
            pub async fn on_device_saved(
                &mut self,
                device_id: String,
                device_description: DeviceWithoutId
            ) -> Result<(), String>;
            pub async fn on_remove_device(&mut self, device_id: String) -> Result<(), String>;
        }
    }

    pub struct MockAdapter {
        adapter_handle: AdapterHandle,
        pub adapter_helper: MockAdapterHelper,
    }

    impl MockAdapter {
        pub fn new(adapter_handle: AdapterHandle) -> Self {
            Self {
                adapter_handle,
                adapter_helper: MockAdapterHelper::new(),
            }
        }
    }

    #[async_trait]
    impl Adapter for MockAdapter {
        fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
            &mut self.adapter_handle
        }

        async fn on_start_pairing(&mut self, timeout: Duration) -> Result<(), String> {
            self.adapter_helper.on_start_pairing(timeout).await
        }

        async fn on_cancel_pairing(&mut self) -> Result<(), String> {
            self.adapter_helper.on_cancel_pairing().await
        }

        async fn on_device_saved(
            &mut self,
            device_id: String,
            device_description: DeviceWithoutId,
        ) -> Result<(), String> {
            self.adapter_helper
                .on_device_saved(device_id, device_description)
                .await
        }

        async fn on_remove_device(&mut self, device_id: String) -> Result<(), String> {
            self.adapter_helper.on_remove_device(device_id).await
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

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";

    #[fixture]
    fn adapter() -> AdapterHandle {
        let client = Arc::new(Mutex::new(Client::new()));
        AdapterHandle::new(client, PLUGIN_ID.to_owned(), ADAPTER_ID.to_owned())
    }

    #[rstest]
    #[tokio::test]
    async fn test_add_device(mut adapter: AdapterHandle) {
        add_mock_device(&mut adapter, DEVICE_ID).await;
        assert!(adapter.get_device(DEVICE_ID).is_some())
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_unknown_device(adapter: AdapterHandle) {
        assert!(adapter.get_device(DEVICE_ID).is_none())
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_device(mut adapter: AdapterHandle) {
        add_mock_device(&mut adapter, DEVICE_ID).await;

        adapter
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

        adapter.remove_device(DEVICE_ID.to_owned()).await.unwrap();

        assert!(adapter.get_device(DEVICE_ID).is_none())
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_unknown_device(mut adapter: AdapterHandle) {
        assert!(adapter.remove_device(DEVICE_ID).await.is_err())
    }

    #[rstest]
    #[tokio::test]
    async fn test_unload(adapter: AdapterHandle) {
        adapter
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

        adapter.unload().await.unwrap();
    }
}
