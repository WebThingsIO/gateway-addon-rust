/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::{
    action::{ActionBase, ActionHandle},
    client::Client,
    device_description::DeviceDescription,
    property::{Property, PropertyBuilder, PropertyHandle},
};
use async_trait::async_trait;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
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
    actions: HashMap<String, Arc<Mutex<Box<dyn ActionBase>>>>,
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
            actions: HashMap::new(),
        }
    }

    pub(crate) fn add_property(&mut self, property_builder: Box<dyn PropertyBuilder>) {
        let description = property_builder.full_description();
        let name = property_builder.name();

        let property_handle = PropertyHandle::new(
            self.client.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.description.id.clone(),
            name.clone(),
            description,
        );

        let property = Arc::new(Mutex::new(property_builder.build(property_handle)));

        self.properties.insert(name, property);
    }

    pub fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn Property>>>> {
        self.properties.get(name).cloned()
    }

    pub(crate) fn add_action(&mut self, action: Box<dyn ActionBase>) {
        let name = action.name();

        let action = Arc::new(Mutex::new(action));

        self.actions.insert(name, action);
    }

    pub fn get_action(&self, name: &str) -> Option<Arc<Mutex<Box<dyn ActionBase>>>> {
        self.actions.get(name).cloned()
    }

    pub(crate) async fn request_action(
        &self,
        action_name: String,
        action_id: String,
        input: Value,
    ) -> Result<(), String> {
        let action = self.get_action(&action_name).ok_or_else(|| {
            format!(
                "Failed to request action {} of {}: not found",
                action_name, self.description.id,
            )
        })?;
        let mut action = action.lock().await;
        let action_handle = ActionHandle::new(
            self.client.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.description.id.clone(),
            action.name(),
            action_id,
            input.clone(),
            input,
        );
        action.check_and_perform(action_handle).await
    }
}

pub trait DeviceBuilder<T: Device> {
    fn build(self, device_handle: DeviceHandle) -> T;
    fn description(&self) -> DeviceDescription;
    fn properties(&self) -> Vec<Box<dyn PropertyBuilder>> {
        Vec::new()
    }
    fn actions(&self) -> Vec<Box<dyn ActionBase>> {
        Vec::new()
    }
    fn id(&self) -> String;
    fn full_description(&self) -> FullDeviceDescription {
        let description = self.description();

        let mut property_descriptions = BTreeMap::new();
        for property_builder in self.properties() {
            property_descriptions
                .insert(property_builder.name(), property_builder.full_description());
        }

        let mut action_descriptions = BTreeMap::new();
        for action in self.actions() {
            action_descriptions.insert(action.name(), action.full_description());
        }

        FullDeviceDescription {
            at_context: description.at_context,
            at_type: description.at_type,
            id: self.id(),
            title: description.title,
            description: description.description,
            properties: Some(property_descriptions),
            actions: Some(action_descriptions),
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
        action::{Action, ActionHandle, NoInput},
        action_description::{ActionDescription, ActionDescriptionBuilder},
        client::MockClient,
        device::DeviceHandle,
        property::{Property, PropertyBuilder, PropertyHandle},
        property_description::{PropertyDescription, PropertyDescriptionBuilder, Type},
    };
    use async_trait::async_trait;
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
            PropertyDescription::default().type_(Type::Integer)
        }

        fn build(self: Box<Self>, property_handle: PropertyHandle) -> Box<dyn Property> {
            Box::new(MockProperty::new(property_handle))
        }

        fn name(&self) -> String {
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

    struct MockAction {
        action_name: String,
    }

    impl MockAction {
        pub fn new(action_name: String) -> Self {
            Self { action_name }
        }
    }

    #[async_trait]
    impl Action for MockAction {
        type Input = NoInput;

        fn name(&self) -> String {
            self.action_name.to_owned()
        }

        fn description(&self) -> ActionDescription<Self::Input> {
            ActionDescription::default()
        }

        async fn perform(
            &mut self,
            _action_handle: ActionHandle<Self::Input>,
        ) -> Result<(), String> {
            Ok(())
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

    #[test]
    fn test_add_action() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let action_name = String::from("action_name");
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

        device.add_action(Box::new(MockAction::new(action_name.to_owned())));

        assert!(device.get_action(&action_name).is_some())
    }
}
