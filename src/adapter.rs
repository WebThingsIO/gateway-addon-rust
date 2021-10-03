/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use crate::client::Client;
use crate::device;
use crate::device::Device;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    AdapterUnloadResponseMessageData, Device as DeviceDescription,
    DeviceAddedNotificationMessageData, DeviceWithoutId, Message,
};

#[async_trait(?Send)]
pub trait Adapter {
    fn get_adapter_handle(&self) -> &AdapterHandle;

    async fn on_device_saved(
        &mut self,
        _id: String,
        _device_description: DeviceWithoutId,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub struct AdapterHandle {
    client: Arc<Mutex<dyn Client>>,
    pub plugin_id: String,
    pub adapter_id: String,
    devices: HashMap<String, Arc<Mutex<dyn Device>>>,
}

impl AdapterHandle {
    pub fn new(client: Arc<Mutex<dyn Client>>, plugin_id: String, adapter_id: String) -> Self {
        Self {
            client,
            plugin_id,
            adapter_id,
            devices: HashMap::new(),
        }
    }

    pub async fn add_device<T, F>(
        &mut self,
        device_description: DeviceDescription,
        constructor: F,
    ) -> Result<Arc<Mutex<T>>, ApiError>
    where
        T: Device + 'static,
        F: FnOnce(device::DeviceHandle) -> T,
    {
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
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            device_description,
        );

        let device = Arc::new(Mutex::new(constructor(device_handle)));

        self.devices.insert(id, device.clone());

        Ok(device)
    }

    pub fn get_device(&self, id: &str) -> Option<Arc<Mutex<dyn Device>>> {
        self.devices.get(id).cloned()
    }

    pub async fn unload(&self) -> Result<(), ApiError> {
        let message: Message = AdapterUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
}

#[cfg(test)]
mod tests {
    use crate::adapter::AdapterHandle;
    use crate::client::MockClient;
    use crate::device::{Device, DeviceHandle};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{Device as DeviceDescription, Message};

    struct MockDevice {
        device_handle: DeviceHandle,
    }

    impl MockDevice {
        pub fn new(device_handle: DeviceHandle) -> Self {
            MockDevice { device_handle }
        }
    }

    impl Device for MockDevice {
        fn borrow_device_handle(&mut self) -> &mut DeviceHandle {
            &mut self.device_handle
        }
    }

    #[tokio::test]
    async fn test_add_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let client = Arc::new(Mutex::new(MockClient::new()));
        let mut adapter = AdapterHandle::new(client.clone(), plugin_id.clone(), adapter_id.clone());

        let description = DeviceDescription {
            at_context: None,
            at_type: None,
            id: device_id.clone(),
            title: None,
            description: None,
            properties: None,
            actions: None,
            events: None,
            links: None,
            base_href: None,
            pin: None,
            credentials_required: None,
        };

        let expected_description = description.clone();

        client
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

        adapter
            .add_device(description, MockDevice::new)
            .await
            .unwrap();

        assert!(adapter.get_device(&device_id).is_some())
    }

    #[tokio::test]
    async fn test_unload() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let client = Arc::new(Mutex::new(MockClient::new()));
        let adapter = AdapterHandle::new(client.clone(), plugin_id.clone(), adapter_id.clone());

        client
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
