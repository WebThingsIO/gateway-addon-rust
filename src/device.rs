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
use webthings_gateway_ipc_types::Device as DeviceDescription;

#[async_trait(?Send)]
pub trait Device: Send {
    async fn init(self: &mut Init<Self>) -> Result<(), String> {
        Ok(())
    }
    fn id(&self) -> &str;
}

pub struct Init<T: ?Sized + Send> {
    device: Box<T>,
    properties: HashMap<String, Box<dyn InitProperty + Send>>,
    description: DeviceDescription,
}

impl<T: ?Sized + Send> Deref for Init<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl<T: ?Sized + Send> DerefMut for Init<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

impl<T: Device + Send> Init<T> {
    pub fn new(device: T) -> Self {
        let id = device.id().to_owned();
        Self {
            device: Box::new(device),
            properties: HashMap::new(),
            description: DeviceDescription {
                id,
                at_context: None,
                at_type: None,
                actions: None,
                base_href: None,
                credentials_required: None,
                description: None,
                events: None,
                links: None,
                pin: None,
                properties: None,
                title: None,
            },
        }
    }

    pub async fn add_property<P: Property + 'static + Send + Sync>(
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

    pub fn add_initialized_property<P: Property + 'static + Send + Sync>(
        &mut self,
        property: property::Init<P>,
    ) {
        self.properties
            .insert(property.id().to_owned(), Box::new(property));
    }

    pub fn description(&self) -> DeviceDescription {
        let properties = self
            .properties
            .iter()
            .map(|(name, property)| (name.clone(), property.description()))
            .collect();
        let mut description = self.description.clone();
        description.properties = Some(properties);
        description.id = self.id().to_owned();
        description
    }

    pub fn description_mut(&mut self) -> &mut DeviceDescription {
        self.description = self.description();
        &mut self.description
    }
}

pub struct Built<T: ?Sized + Send> {
    device: Box<T>,
    client: Arc<Mutex<Client>>,
    plugin_id: String,
    adapter_id: String,
    properties: HashMap<String, Arc<Mutex<Box<dyn BuiltProperty + Send>>>>,
    description: DeviceDescription,
}

impl<T: Device + 'static + Send> Built<T> {
    pub(crate) fn new(
        device: Init<T>,
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
    ) -> Self {
        let client_copy = client.clone();
        let plugin_id_copy = plugin_id.clone();
        let adapter_id_copy = adapter_id.clone();
        let device_id = device.id().to_owned();
        let description = device.description;
        let Init {
            device,
            properties,
            description: _,
        } = device;
        let properties: HashMap<String, Arc<Mutex<Box<dyn BuiltProperty + Send + 'static>>>> =
            properties
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
            description,
        }
    }
}

impl<T: Device + Send> Deref for Built<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl<T: Device + Send> DerefMut for Built<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

pub trait BuiltDevice: Send {
    fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn BuiltProperty + Send>>>>;
    fn description(&self) -> &DeviceDescription;
    fn description_mut(&mut self) -> &mut DeviceDescription;
}

impl<T: Device + Send> BuiltDevice for Built<T> {
    fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn BuiltProperty + Send>>>> {
        self.properties.get(name).cloned()
    }

    fn description(&self) -> &DeviceDescription {
        &self.description
    }

    fn description_mut(&mut self) -> &mut DeviceDescription {
        &mut self.description
    }
}
