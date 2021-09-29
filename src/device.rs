/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::client::Client;
use crate::property::{Property, PropertyHandle};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{Device as DeviceDescription, Property as PropertyDescription};

#[async_trait(?Send)]
pub trait Device {
    fn borrow_device_handle(&mut self) -> &mut DeviceHandle;
}

#[derive(Clone)]
pub struct DeviceHandle {
    client: Arc<Mutex<Client>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub description: DeviceDescription,
    properties: HashMap<String, Arc<Mutex<dyn Property>>>,
}

impl DeviceHandle {
    pub fn new(
        client: Arc<Mutex<Client>>,
        plugin_id: String,
        adapter_id: String,
        description: DeviceDescription,
    ) -> Self {
        DeviceHandle {
            client,
            plugin_id,
            adapter_id,
            description,
            properties: HashMap::new(),
        }
    }

    pub fn add_property<T, F>(
        &mut self,
        name: String,
        description: PropertyDescription,
        constructor: F,
    ) where
        T: Property + 'static,
        F: FnOnce(PropertyHandle) -> T,
    {
        let property_handle = PropertyHandle::new(
            self.client.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.description.id.clone(),
            name.clone(),
            description,
        );

        let property = Arc::new(Mutex::new(constructor(property_handle)));

        self.properties.insert(name, property);
    }

    pub fn get_property(&self, name: &str) -> Option<Arc<Mutex<dyn Property>>> {
        self.properties.get(name).cloned()
    }
}
