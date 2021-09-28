/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::{
    api_error::ApiError,
    client::Client,
    property::{self, BuiltProperty, InitProperty, Property},
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{Device as FullDeviceDescription, DevicePin, Link};
pub struct DeviceDescription {
    pub at_context: Option<String>,
    pub at_type: Option<Vec<String>>,
    pub base_href: Option<String>,
    pub credentials_required: Option<bool>,
    pub description: Option<String>,
    pub id: String,
    pub links: Option<Vec<Link>>,
    pub pin: Option<DevicePin>,
    pub title: Option<String>,
}

#[async_trait(?Send)]
pub trait Device {
    fn description(&self) -> DeviceDescription;
    async fn init(self: &mut Init<Self>) -> Result<(), String> {
        Ok(())
    }
}

pub struct Init<T: ?Sized> {
    device: Box<T>,
    properties: HashMap<String, Box<dyn InitProperty>>,
}

impl<T: ?Sized> Deref for Init<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl<T: ?Sized> DerefMut for Init<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

impl<T: Device> Init<T> {
    pub fn new(device: T) -> Self {
        Self {
            device: Box::new(device),
            properties: HashMap::new(),
        }
    }

    pub async fn add_property<P: Property + 'static>(
        &mut self,
        property: P,
    ) -> Result<(), ApiError> {
        let mut property = property::Init::new(property);
        property
            .init()
            .await
            .map_err(|err| ApiError::InitializeProperty(err))?;
        self.add_initialized_property(property);
        Ok(())
    }

    pub fn add_initialized_property<P: Property + 'static>(&mut self, property: property::Init<P>) {
        self.properties
            .insert(property.name().to_owned(), Box::new(property));
    }

    pub fn full_description(&self) -> FullDeviceDescription {
        let properties = self
            .properties
            .iter()
            .map(|(name, property)| (name.clone(), property.description()))
            .collect();
        let description = self.description();
        FullDeviceDescription {
            at_context: description.at_context.clone(),
            at_type: description.at_type.clone(),
            actions: None.clone(),
            base_href: description.base_href.clone(),
            credentials_required: description.credentials_required.clone(),
            description: description.description.clone(),
            events: None.clone(),
            id: description.id.clone(),
            links: description.links.clone(),
            pin: description.pin.clone(),
            properties: Some(properties).clone(),
            title: description.title.clone(),
        }
    }
}

pub struct Built<T: ?Sized> {
    device: Box<T>,
    client: Arc<Mutex<Client>>,
    plugin_id: String,
    adapter_id: String,
    properties: HashMap<String, Arc<Mutex<Box<dyn BuiltProperty>>>>,
}

impl<T: Device + 'static> Built<T> {
    pub(crate) fn new(
        device: Init<T>,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
    ) -> Self {
        let client_copy = client.clone();
        let plugin_id_copy = plugin_id.clone();
        let adapter_id_copy = adapter_id.clone();
        let device_id = device.description().id.clone();
        let Init { device, properties } = device;
        let properties: HashMap<String, Arc<Mutex<Box<dyn BuiltProperty>>>> = properties
            .into_iter()
            .map(move |(name, property)| {
                let property = property.into_built(
                    client_copy.clone(),
                    plugin_id_copy.clone(),
                    adapter_id_copy.clone(),
                    device_id.clone(),
                );
                (name.clone(), Arc::new(Mutex::new(property)))
            })
            .collect();
        Self {
            device,
            client,
            plugin_id,
            adapter_id,
            properties,
        }
    }
}

impl<T: Device> Deref for Built<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl<T: Device> DerefMut for Built<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

pub trait BuiltDevice {
    fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn BuiltProperty>>>>;
}

impl<T: Device> BuiltDevice for Built<T> {
    fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn BuiltProperty>>>> {
        self.properties.get(name).cloned()
    }
}
