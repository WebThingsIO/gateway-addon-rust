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
    client: Arc<Mutex<Client>>,
    pub plugin_id: String,
    pub adapter_id: String,
    devices: HashMap<String, Arc<Mutex<dyn Device>>>,
}

impl AdapterHandle {
    pub fn new(client: Arc<Mutex<Client>>, plugin_id: String, adapter_id: String) -> Self {
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
