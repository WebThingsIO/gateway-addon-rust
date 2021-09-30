/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use crate::client::Client;
use crate::device::{self, BuiltDevice, Device};
use async_trait::async_trait;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    AdapterUnloadResponseMessageData, DeviceAddedNotificationMessageData, DeviceWithoutId, Message,
};

#[async_trait(?Send)]
pub trait Adapter: Send {
    fn id(&self) -> &str;
    fn name(&self) -> &str;

    async fn init(self: &mut Built<Self>) -> Result<(), String> {
        Ok(())
    }

    async fn on_device_saved(
        self: &Built<Self>,
        _id: String,
        _device_description: DeviceWithoutId,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub struct Built<T: ?Sized> {
    adapter: Box<T>,
    client: Arc<Mutex<Client>>,
    plugin_id: String,
    adapter_id: String,
    devices: HashMap<String, Arc<Mutex<dyn BuiltDevice + Send>>>,
}

impl<T> Built<T> {
    pub(crate) fn new(
        adapter: T,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
    ) -> Self {
        Self {
            adapter: Box::new(adapter),
            client,
            plugin_id,
            adapter_id,
            devices: HashMap::new(),
        }
    }

    pub async fn add_device<D: Device + 'static + Send>(
        &mut self,
        device: D,
    ) -> Result<Arc<Mutex<device::Built<D>>>, ApiError> {
        let mut device = device::Init::new(device);

        device
            .init()
            .await
            .map_err(|err| ApiError::InitializeDevice(err))?;

        self.add_initialized_device(device).await
    }

    pub async fn add_initialized_device<D: Device + 'static + Send>(
        &mut self,
        device: device::Init<D>,
    ) -> Result<Arc<Mutex<device::Built<D>>>, ApiError> {
        let message: Message = DeviceAddedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device: device.description(),
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        let id = device.id().to_owned();

        let device = Arc::new(Mutex::new(device::Built::new(
            device,
            self.client.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
        )));

        self.devices.insert(id, device.clone());

        Ok(device)
    }
}

impl<T: ?Sized> Deref for Built<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.adapter
    }
}

impl<T: ?Sized> DerefMut for Built<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.adapter
    }
}

#[async_trait(?Send)]
pub trait BuiltAdapter: Send {
    fn get_device(&self, id: &str) -> Option<Arc<Mutex<dyn BuiltDevice + Send>>>;

    async fn unload(&self) -> Result<(), ApiError>;

    async fn on_device_saved(
        &mut self,
        _id: String,
        _device_description: DeviceWithoutId,
    ) -> Result<(), String>;
}

#[async_trait(?Send)]
impl<T> BuiltAdapter for Built<T>
where
    T: Adapter,
{
    fn get_device(&self, id: &str) -> Option<Arc<Mutex<dyn BuiltDevice + Send>>> {
        self.devices.get(id).cloned()
    }

    async fn unload(&self) -> Result<(), ApiError> {
        let message: Message = AdapterUnloadResponseMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }

    async fn on_device_saved(
        &mut self,
        id: String,
        device_description: DeviceWithoutId,
    ) -> Result<(), String> {
        Adapter::on_device_saved(self, id, device_description).await
    }
}
