/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use crate::client::Client;
use crate::device;
use crate::device::{Device, DeviceHandle};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    AdapterRemoveDeviceResponseMessageData, AdapterUnloadResponseMessageData,
    Device as DeviceDescription, DeviceAddedNotificationMessageData, DeviceWithoutId, Message,
};

#[async_trait]
pub trait Adapter: Send {
    fn get_adapter_handle(&mut self) -> &mut AdapterHandle;

    async fn on_device_saved(
        &mut self,
        _id: String,
        _device_description: DeviceWithoutId,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn on_start_pairing(&mut self, _timeout: Duration) -> Result<(), String> {
        Ok(())
    }

    async fn on_cancel_pairing(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn on_remove_device(&mut self, _device_id: String) -> Result<(), String> {
        Ok(())
    }
}

pub trait DeviceBuilder<T: Device> {
    fn build(self, device_handle: DeviceHandle) -> T;
    fn description(&self) -> DeviceDescription;
}

#[derive(Clone)]
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

    pub async fn add_device<D, B>(&mut self, device_builder: B) -> Result<Arc<Mutex<D>>, ApiError>
    where
        D: Device + 'static,
        B: DeviceBuilder<D>,
    {
        let device_description = device_builder.description();

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

        let device = Arc::new(Mutex::new(device_builder.build(device_handle)));

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

    pub async fn remove_device(&mut self, device_id: &str) -> Result<(), String> {
        self.devices.remove(device_id);

        let message: Message = AdapterRemoveDeviceResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: device_id.to_owned(),
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
mod tests {
    use crate::{
        adapter::{Adapter, AdapterHandle, DeviceBuilder},
        client::MockClient,
        device::{Device, DeviceHandle},
        plugin::{connect, Plugin},
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{
        AdapterRemoveDeviceRequestMessageData, Device as DeviceDescription, Message,
    };

    struct MockAdapter {
        adapter_handle: AdapterHandle,
    }

    impl MockAdapter {
        pub fn new(adapter_handle: AdapterHandle) -> Self {
            Self { adapter_handle }
        }
    }

    impl Adapter for MockAdapter {
        fn get_adapter_handle(&mut self) -> &mut AdapterHandle {
            &mut self.adapter_handle
        }
    }

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

    struct MockDeviceBuilder {
        device_id: String,
    }

    impl MockDeviceBuilder {
        pub fn new(device_id: String) -> Self {
            Self { device_id }
        }
    }

    impl DeviceBuilder<MockDevice> for MockDeviceBuilder {
        fn build(self, device_handle: DeviceHandle) -> MockDevice {
            MockDevice::new(device_handle)
        }

        fn description(&self) -> DeviceDescription {
            DeviceDescription {
                at_context: None,
                at_type: None,
                id: self.device_id.clone(),
                title: None,
                description: None,
                properties: None,
                actions: None,
                events: None,
                links: None,
                base_href: None,
                pin: None,
                credentials_required: None,
            }
        }
    }

    async fn create_mock_adapter(
        plugin: &mut Plugin,
        client: Arc<Mutex<MockClient>>,
        adapter_id: &str,
    ) -> Arc<Mutex<MockAdapter>> {
        let plugin_id = plugin.plugin_id.to_owned();
        let adapter_id_copy = adapter_id.to_owned();

        client
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
            .create_adapter(&adapter_id, &adapter_id, |adapter| {
                MockAdapter::new(adapter)
            })
            .await
            .unwrap()
    }

    async fn create_mock_device(
        adapter: Arc<Mutex<MockAdapter>>,
        client: Arc<Mutex<MockClient>>,
        device_id: &str,
    ) -> Arc<Mutex<MockDevice>> {
        let device_builder = MockDeviceBuilder::new(device_id.to_owned());
        let expected_description = device_builder.description();

        let adapter = &mut adapter.lock().await.adapter_handle;
        let plugin_id = adapter.plugin_id.to_owned();
        let adapter_id = adapter.adapter_id.to_owned();

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

        adapter.add_device(device_builder).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_adapter() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let (mut plugin, client) = connect(&plugin_id);
        create_mock_adapter(&mut plugin, client, &adapter_id).await;
        assert!(plugin.borrow_adapter(&adapter_id).is_ok());
    }

    #[tokio::test]
    async fn test_add_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let (mut plugin, client) = connect(&plugin_id);
        let adapter = create_mock_adapter(&mut plugin, client.clone(), &adapter_id).await;
        create_mock_device(adapter.clone(), client, &device_id).await;

        assert!(adapter
            .lock()
            .await
            .adapter_handle
            .get_device(&device_id)
            .is_some())
    }

    #[tokio::test]
    async fn test_remove_device() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let device_id_copy = device_id.clone();
        let (mut plugin, client) = connect(&plugin_id);
        let adapter = create_mock_adapter(&mut plugin, client.clone(), &adapter_id).await;
        create_mock_device(adapter.clone(), client.clone(), &device_id).await;

        let message: Message = AdapterRemoveDeviceRequestMessageData {
            device_id: device_id.clone(),
            plugin_id: plugin_id.clone(),
            adapter_id: adapter_id.to_owned(),
        }
        .into();

        client
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

        plugin
            .handle_message(message)
            .await
            .expect("Handle message");

        assert!(adapter
            .lock()
            .await
            .adapter_handle
            .get_device(&device_id_copy)
            .is_none())
    }

    #[tokio::test]
    async fn test_unload() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");

        let (mut plugin, client) = connect(&plugin_id);
        let adapter = create_mock_adapter(&mut plugin, client.clone(), &adapter_id).await;

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

        adapter.lock().await.adapter_handle.unload().await.unwrap();
    }
}
