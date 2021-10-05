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

#[async_trait]
pub trait Device {
    fn borrow_device_handle(&mut self) -> &mut DeviceHandle;
}

#[derive(Clone)]
pub struct DeviceHandle {
    client: Arc<Mutex<dyn Client>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub description: DeviceDescription,
    properties: HashMap<String, Arc<Mutex<dyn Property>>>,
}

impl DeviceHandle {
    pub fn new(
        client: Arc<Mutex<dyn Client>>,
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

#[cfg(test)]
mod tests {
    use crate::client::MockClient;
    use crate::device::DeviceHandle;
    use crate::property::{Property, PropertyHandle};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{
        Device as DeviceDescription, Property as PropertyDescription,
    };

    struct MockProperty {
        property_handle: PropertyHandle,
    }

    impl MockProperty {
        pub fn new(property_handle: PropertyHandle) -> Self {
            MockProperty { property_handle }
        }
    }

    impl Property for MockProperty {
        fn borrow_property_handle(&mut self) -> &mut PropertyHandle {
            &mut self.property_handle
        }
    }

    #[test]
    fn test_add_property() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let client = Arc::new(Mutex::new(MockClient::new()));

        let device_description = DeviceDescription {
            at_context: None,
            at_type: None,
            id: device_id,
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

        let mut device = DeviceHandle::new(client, plugin_id, adapter_id, device_description);

        let property_description = PropertyDescription {
            at_type: None,
            name: Some(property_name.clone()),
            title: None,
            description: None,
            type_: String::from("integer"),
            unit: None,
            enum_: None,
            links: None,
            minimum: None,
            maximum: None,
            multiple_of: None,
            read_only: None,
            value: None,
            visible: None,
        };

        device.add_property(
            property_name.clone(),
            property_description,
            MockProperty::new,
        );

        assert!(device.get_property(&property_name).is_some())
    }
}
