/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::client::Client;
use crate::device_description::DeviceDescription;
use crate::property::{Property, PropertyBuilder, PropertyHandle};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::Device as FullDeviceDescription;

#[async_trait]
pub trait Device: Send {
    fn device_handle_mut(&mut self) -> &mut DeviceHandle;
}

#[derive(Clone)]
pub struct DeviceHandle {
    client: Arc<Mutex<dyn Client>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub description: FullDeviceDescription,
    properties: HashMap<String, Arc<Mutex<Box<dyn Property>>>>,
}

impl DeviceHandle {
    pub fn new(
        client: Arc<Mutex<dyn Client>>,
        plugin_id: String,
        adapter_id: String,
        description: FullDeviceDescription,
    ) -> Self {
        DeviceHandle {
            client,
            plugin_id,
            adapter_id,
            description,
            properties: HashMap::new(),
        }
    }

    pub(crate) fn add_property(&mut self, property_builder: Box<dyn PropertyBuilder>) {
        let description = property_builder.description();
        let id = property_builder.id();

        let property_handle = PropertyHandle::new(
            self.client.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.description.id.clone(),
            id.clone(),
            description,
        );

        let property = Arc::new(Mutex::new(property_builder.build(property_handle)));

        self.properties.insert(id, property);
    }

    pub fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn Property>>>> {
        self.properties.get(name).cloned()
    }
}

pub trait DeviceBuilder<T: Device> {
    fn build(self, device_handle: DeviceHandle) -> T;
    fn description(&self) -> DeviceDescription;
    fn properties(&self) -> Vec<Box<dyn PropertyBuilder>>;
    fn id(&self) -> String;
    fn full_description(&self) -> FullDeviceDescription {
        let description = self.description();

        let mut property_descriptions = BTreeMap::new();
        for property_builder in self.properties() {
            property_descriptions.insert(property_builder.id(), property_builder.description());
        }

        FullDeviceDescription {
            at_context: description.at_context,
            at_type: description.at_type,
            id: self.id(),
            title: description.title,
            description: description.description,
            properties: Some(property_descriptions),
            actions: None,
            events: None,
            links: description.links,
            base_href: description.base_href,
            pin: description.pin,
            credentials_required: description.credentials_required,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client::MockClient,
        device::DeviceHandle,
        property::{Property, PropertyBuilder, PropertyHandle},
        property_description::{PropertyDescription, PropertyDescriptionBuilder, Type},
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Device as DeviceDescription;

    struct MockPropertyBuilder {
        property_name: String,
    }

    impl MockPropertyBuilder {
        pub fn new(property_name: String) -> Self {
            Self { property_name }
        }
    }

    impl PropertyBuilder for MockPropertyBuilder {
        fn description(&self) -> PropertyDescription {
            PropertyDescription::default()
                .name(&self.property_name)
                .type_(Type::Integer)
        }

        fn build(self: Box<Self>, property_handle: PropertyHandle) -> Box<dyn Property> {
            Box::new(MockProperty::new(property_handle))
        }

        fn id(&self) -> String {
            self.property_name.to_owned()
        }
    }

    struct MockProperty {
        property_handle: PropertyHandle,
    }

    impl MockProperty {
        pub fn new(property_handle: PropertyHandle) -> Self {
            MockProperty { property_handle }
        }
    }

    impl Property for MockProperty {
        fn property_handle_mut(&mut self) -> &mut PropertyHandle {
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

        device.add_property(Box::new(MockPropertyBuilder::new(property_name.clone())));

        assert!(device.get_property(&property_name).is_some())
    }
}
