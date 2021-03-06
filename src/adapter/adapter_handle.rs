/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    client::Client, device::DeviceBuilder, error::WebthingsError, Adapter, Device, DeviceHandle,
};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    AdapterRemoveDeviceResponseMessageData, AdapterUnloadResponseMessageData,
    DeviceAddedNotificationMessageData, Message,
};

/// A struct which represents an instance of a WebthingsIO adapter.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct AdapterHandle {
    pub(crate) client: Arc<Mutex<Client>>,
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

    /// Build and add a new device using the given data struct.
    pub async fn add_device<D: DeviceBuilder>(
        &mut self,
        device: D,
    ) -> Result<Arc<Mutex<Box<dyn Device>>>, WebthingsError> {
        let device_description = device.full_description()?;

        let message: Message = DeviceAddedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device: device_description.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        let id = device_description.id.clone();

        let device_handle = DeviceHandle::new(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            device.id(),
            device.description(),
        );

        let properties = device.properties();
        let actions = device.actions();
        let events = device.events();

        let device: Arc<Mutex<Box<dyn Device>>> =
            Arc::new(Mutex::new(Box::new(D::build(device, device_handle))));
        let device_weak = Arc::downgrade(&device);

        {
            let mut device = device.lock().await;
            let device_handle = device.device_handle_mut();
            device_handle.weak = device_weak;

            for property_builder in properties {
                device_handle.add_property(property_builder).await;
            }

            for action in actions {
                device_handle.add_action(action).await;
            }

            for event_builder in events {
                device_handle.add_event(event_builder).await;
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
        client::Client,
        device::{tests::MockDevice, DeviceStructure},
        AdapterHandle, Device,
    };
    use rstest::{fixture, rstest};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    pub async fn add_mock_device(
        adapter: &mut AdapterHandle,
        device_id: &str,
    ) -> Arc<Mutex<Box<dyn Device>>> {
        let device = MockDevice::new(device_id.to_owned());
        let expected_description = device.full_description().unwrap();

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

        adapter.add_device(device).await.unwrap()
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
