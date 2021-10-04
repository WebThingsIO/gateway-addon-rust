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
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DevicePropertyChangedNotificationMessageData, Message, Property as PropertyDescription,
};

pub enum Type {
    Null,
    Boolean,
    Integer,
    Number,
    String,
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Null => "null",
            Type::Boolean => "boolean",
            Type::Integer => "integer",
            Type::Number => "number",
            Type::String => "string",
        }
        .to_owned()
    }
}

#[async_trait]
pub trait Property: Send + Sync {
    async fn on_update(self: &Built<Self>, _value: Value) -> Result<(), String> {
        Ok(())
    }

    async fn init(self: &mut Init<Self>) -> Result<(), String> {
        Ok(())
    }

    async fn built(self: &mut Built<Self>) -> Result<(), String> {
        Ok(())
    }

    fn id(&self) -> &str;

    fn type_(&self) -> Type;
}

pub struct Init<T: ?Sized + Send> {
    property: Box<T>,
    description: PropertyDescription,
}

impl<T: Property + Send> Init<T> {
    pub fn new(property: T) -> Self {
        let type_ = property.type_().to_string();
        Self {
            property: Box::new(property),
            description: PropertyDescription {
                at_type: None,
                description: None,
                enum_: None,
                links: None,
                maximum: None,
                minimum: None,
                multiple_of: None,
                name: None,
                read_only: None,
                title: None,
                type_: type_,
                unit: None,
                value: None,
                visible: None,
            },
        }
    }
}

impl<T: ?Sized + Send> Deref for Init<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.property
    }
}

impl<T: ?Sized + Send> DerefMut for Init<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.property
    }
}

pub trait InitProperty: Send {
    fn into_built(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Arc<Mutex<dyn BuiltProperty + Send + 'static>>;

    fn description(&self) -> PropertyDescription;
    fn description_mut(&mut self) -> &mut PropertyDescription;
}

impl<T: Property + 'static + Send + Sync> InitProperty for Init<T> {
    fn into_built(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Arc<Mutex<dyn BuiltProperty + Send + 'static>> {
        Built::new(*self, client, plugin_id, adapter_id, device_id)
    }

    fn description(&self) -> PropertyDescription {
        let mut description = self.description.clone();
        description.type_ = self.type_().to_string();
        description
    }

    fn description_mut(&mut self) -> &mut PropertyDescription {
        self.description = self.description();
        &mut self.description
    }
}

pub struct Built<T: ?Sized + Send + Sync> {
    property: Box<T>,
    client: Arc<Mutex<Client>>,
    plugin_id: String,
    adapter_id: String,
    device_id: String,
    description: PropertyDescription,
    weak: Weak<Mutex<Built<T>>>,
}

impl<T: Property + 'static + Send + Sync> Built<T> {
    pub(crate) fn new(
        property: Init<T>,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Arc<Mutex<Self>> {
        let description = property.description();
        let Init {
            property,
            description: _,
        } = property;
        Arc::new_cyclic(|weak| {
            Mutex::new(Self {
                property,
                client,
                plugin_id,
                adapter_id,
                device_id,
                description,
                weak: weak.clone(),
            })
        })
    }

    pub fn description(&self) -> &PropertyDescription {
        &self.description
    }

    pub fn weak(&self) -> &Weak<Mutex<Built<T>>> {
        &self.weak
    }
}
impl<T: ?Sized + Send + Sync> Deref for Built<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.property
    }
}

impl<T: ?Sized + Send + Sync> DerefMut for Built<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.property
    }
}

#[async_trait]
pub trait BuiltProperty: Send {
    fn set_cached_value(&mut self, value: Value);
    async fn set_value(&mut self, value: Value) -> Result<(), ApiError>;
    async fn on_update(&mut self, value: Value) -> Result<(), String>;
    async fn built(&mut self) -> Result<(), String>;
}

#[async_trait]
impl<T: Property + 'static + Send + Sync> BuiltProperty for Built<T> {
    fn set_cached_value(&mut self, value: Value) {
        self.description.value = Some(value);
    }

    async fn set_value(&mut self, value: Value) -> Result<(), ApiError> {
        self.set_cached_value(value);
        let description = self.description().clone();

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

    async fn built(&mut self) -> Result<(), String> {
        Property::built(self).await
    }
}
