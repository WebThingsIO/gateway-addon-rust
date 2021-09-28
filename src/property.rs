/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use crate::client::Client;
use async_trait::async_trait;
use serde_json::Value;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DevicePropertyChangedNotificationMessageData, Message, Property as PropertyDescription,
};

#[async_trait(?Send)]
pub trait Property {
    async fn on_update(self: &Built<Self>, _value: Value) -> Result<(), String> {
        Ok(())
    }
    async fn init(self: &mut Init<Self>) -> Result<(), String> {
        Ok(())
    }
    fn description(&self) -> PropertyDescription;
    fn name(&self) -> &str;
}

pub struct Init<T: ?Sized> {
    property: Box<T>,
}

impl<T: Property> Init<T> {
    pub fn new(property: T) -> Self {
        Self {
            property: Box::new(property),
        }
    }
}

impl<T: ?Sized> Deref for Init<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.property
    }
}

impl<T: ?Sized> DerefMut for Init<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.property
    }
}

pub trait InitProperty {
    fn into_built(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn BuiltProperty>;

    fn description(&self) -> PropertyDescription;
}

impl<T: Property + 'static> InitProperty for Init<T> {
    fn into_built(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn BuiltProperty> {
        Box::new(Built::new(
            *self.property,
            client,
            plugin_id,
            adapter_id,
            device_id,
        ))
    }

    fn description(&self) -> PropertyDescription {
        self.property.description()
    }
}

pub struct Built<T: ?Sized> {
    property: Box<T>,
    client: Arc<Mutex<Client>>,
    plugin_id: String,
    adapter_id: String,
    device_id: String,
}

impl<T: Property> Built<T> {
    pub(crate) fn new(
        property: T,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Self {
        Self {
            property: Box::new(property),
            client,
            plugin_id,
            adapter_id,
            device_id,
        }
    }
}

impl<T: ?Sized> Deref for Built<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.property
    }
}

impl<T: ?Sized> DerefMut for Built<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.property
    }
}

#[async_trait(?Send)]
pub trait BuiltProperty {
    async fn set_value(&mut self, value: Value) -> Result<(), ApiError>;
    async fn on_update(&mut self, _value: Value) -> Result<(), String>;
}

#[async_trait(?Send)]
impl<T: Property> BuiltProperty for Built<T> {
    async fn set_value(&mut self, value: Value) -> Result<(), ApiError> {
        let mut description = self.description().clone();
        description.value = Some(value);

        let message: Message = DevicePropertyChangedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: self.device_id.clone(),
            property: description,
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
    async fn on_update(&mut self, value: Value) -> Result<(), String> {
        Property::on_update(self, value).await
    }
}
