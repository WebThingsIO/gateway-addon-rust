/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::{
    action::{ActionBase, ActionHandle},
    adapter::AdapterBase,
    api_error::ApiError,
    client::Client,
    device_description::DeviceDescription,
    event::{EventBase, EventHandleBase},
    property::{PropertyBase, PropertyBuilderBase},
};
use async_trait::async_trait;
use serde_json::Value;
use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::Device as FullDeviceDescription;

#[async_trait]
pub trait Device: Send + Sized + 'static {
    fn device_handle_mut(&mut self) -> &mut DeviceHandle;
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
pub trait DeviceBase: Send {
    fn device_handle_mut(&mut self) -> &mut DeviceHandle;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[async_trait]
impl<T: Device> DeviceBase for T {
    fn device_handle_mut(&mut self) -> &mut DeviceHandle {
        T::device_handle_mut(self)
    }
    fn as_any(&self) -> &dyn Any {
        T::as_any(self)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        T::as_any_mut(self)
    }
}

#[derive(Clone)]
pub struct DeviceHandle {
    client: Arc<Mutex<dyn Client>>,
    pub(crate) weak: Weak<Mutex<Box<dyn DeviceBase>>>,
    pub adapter: Weak<Mutex<Box<dyn AdapterBase>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub description: FullDeviceDescription,
    properties: HashMap<String, Arc<Mutex<Box<dyn PropertyBase>>>>,
    actions: HashMap<String, Arc<Mutex<Box<dyn ActionBase>>>>,
    events: HashMap<String, Arc<Mutex<Box<dyn EventHandleBase>>>>,
}

impl DeviceHandle {
    pub(crate) fn new(
        client: Arc<Mutex<dyn Client>>,
        adapter: Weak<Mutex<Box<dyn AdapterBase>>>,
        plugin_id: String,
        adapter_id: String,
        description: FullDeviceDescription,
    ) -> Self {
        DeviceHandle {
            client,
            weak: Weak::new(),
            adapter,
            plugin_id,
            adapter_id,
            description,
            properties: HashMap::new(),
            actions: HashMap::new(),
            events: HashMap::new(),
        }
    }

    pub(crate) fn add_property(&mut self, property_builder: Box<dyn PropertyBuilderBase>) {
        let description = property_builder.full_description();
        let name = property_builder.name();

        let property = Arc::new(Mutex::new(property_builder.build(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.description.id.clone(),
            name.clone(),
            description,
        )));

        self.properties.insert(name, property);
    }

    pub fn properties(&self) -> &HashMap<String, Arc<Mutex<Box<dyn PropertyBase>>>> {
        &self.properties
    }

    pub fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn PropertyBase>>>> {
        self.properties.get(name).cloned()
    }

    pub async fn set_property_value(&self, name: &str, data: Value) -> Result<(), ApiError> {
        if let Some(property) = self.properties.get(name) {
            let mut property = property.lock().await;
            property.property_handle_mut().set_value(data).await?;
            Ok(())
        } else {
            Err(ApiError::UnknownProperty)
        }
    }

    pub(crate) fn add_action(&mut self, action: Box<dyn ActionBase>) {
        let name = action.name();

        let action = Arc::new(Mutex::new(action));

        self.actions.insert(name, action);
    }

    pub fn actions(&self) -> &HashMap<String, Arc<Mutex<Box<dyn ActionBase>>>> {
        &self.actions
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
            self.weak.clone(),
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

    pub(crate) fn add_event(&mut self, event: Box<dyn EventBase>) {
        let name = event.name();

        let event_handle = event.build_event_handle(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.description.id.clone(),
            name.clone(),
        );

        let event = Arc::new(Mutex::new(event_handle));

        self.events.insert(name, event);
    }

    pub fn events(&self) -> &HashMap<String, Arc<Mutex<Box<dyn EventHandleBase>>>> {
        &self.events
    }

    pub fn get_event(&self, name: &str) -> Option<Arc<Mutex<Box<dyn EventHandleBase>>>> {
        self.events.get(name).cloned()
    }

    pub async fn raise_event(&self, name: &str, data: Option<Value>) -> Result<(), ApiError> {
        if let Some(event) = self.events.get(name) {
            let event = event.lock().await;
            event.raise(data).await?;
            Ok(())
        } else {
            Err(ApiError::UnknownEvent)
        }
    }
}

pub trait DeviceBuilder {
    type Device: Device;
    fn id(&self) -> String;
    fn description(&self) -> DeviceDescription;
    fn properties(&self) -> Vec<Box<dyn PropertyBuilderBase>> {
        Vec::new()
    }
    fn actions(&self) -> Vec<Box<dyn ActionBase>> {
        Vec::new()
    }
    fn events(&self) -> Vec<Box<dyn EventBase>> {
        Vec::new()
    }
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

        let mut event_descriptions = BTreeMap::new();
        for event in self.events() {
            event_descriptions.insert(event.name(), event.full_description());
        }

        FullDeviceDescription {
            at_context: description.at_context,
            at_type: description.at_type,
            id: self.id(),
            title: description.title,
            description: description.description,
            properties: Some(property_descriptions),
            actions: Some(action_descriptions),
            events: Some(event_descriptions),
            links: description.links,
            base_href: description.base_href,
            pin: description.pin,
            credentials_required: description.credentials_required,
        }
    }
    fn build(self, device_handle: DeviceHandle) -> Self::Device;
}

#[cfg(test)]
mod tests {
    use crate::{
        action::{Action, ActionHandle, NoInput},
        action_description::ActionDescription,
        client::MockClient,
        device::DeviceHandle,
        event::Event,
        event_description::EventDescription,
        property::{Property, PropertyBuilder, PropertyHandle},
        property_description::PropertyDescription,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Weak};
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
        type Property = MockProperty;
        type Value = i32;

        fn description(&self) -> PropertyDescription<Self::Value> {
            PropertyDescription::default()
        }

        fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property {
            MockProperty::new(property_handle)
        }

        fn name(&self) -> String {
            self.property_name.to_owned()
        }
    }

    struct MockProperty {
        property_handle: PropertyHandle<i32>,
    }

    impl MockProperty {
        pub fn new(property_handle: PropertyHandle<i32>) -> Self {
            MockProperty { property_handle }
        }
    }

    impl Property for MockProperty {
        type Value = i32;
        fn property_handle_mut(&mut self) -> &mut PropertyHandle<i32> {
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

    struct MockEvent {
        event_name: String,
    }

    impl MockEvent {
        pub fn new(event_name: String) -> Self {
            Self { event_name }
        }
    }

    impl Event for MockEvent {
        type Data = u32;

        fn name(&self) -> String {
            self.event_name.clone()
        }

        fn description(&self) -> EventDescription<Self::Data> {
            todo!()
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

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_description,
        );

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

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_description,
        );

        device.add_action(Box::new(MockAction::new(action_name.to_owned())));

        assert!(device.get_action(&action_name).is_some())
    }

    #[test]
    fn test_add_event() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let event_name = String::from("event_name");
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

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_description,
        );

        device.add_event(Box::new(MockEvent::new(event_name.to_owned())));

        assert!(device.get_event(&event_name).is_some())
    }
}
