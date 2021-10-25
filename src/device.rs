/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WoT devices aka things.

pub use crate::device_description::*;
use crate::{
    action::{ActionBase, ActionHandle, Actions},
    actions,
    adapter::Adapter,
    api_error::ApiError,
    client::Client,
    event::{EventBase, EventHandleBase, Events},
    events, properties,
    property::{Properties, PropertyBase, PropertyBuilderBase},
};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::Device as FullDeviceDescription;

/// A trait used to specify the behaviour of a WoT device.
///
/// Wraps a [device handle][DeviceHandle] and defines how to react on gateway requests. Built by a [device builder][DeviceBuilder].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::prelude::*;
/// struct ExampleDevice(DeviceHandle);
///
/// impl Device for ExampleDevice {
///     fn device_handle_mut(&mut self) -> &mut DeviceHandle {
///         &mut self.0
///     }
/// }
/// ```
#[async_trait]
pub trait Device: Send + Sync + AsAny + 'static {
    /// Return the wrapped [device handle][DeviceHandle].
    fn device_handle_mut(&mut self) -> &mut DeviceHandle;
}

impl Downcast for dyn Device {}

/// A struct which represents an instance of a WoT device.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct DeviceHandle {
    client: Arc<Mutex<dyn Client>>,
    pub(crate) weak: Weak<Mutex<Box<dyn Device>>>,
    /// Reference to the [adapter][crate::adapter::Adapter] which owns this device.
    pub adapter: Weak<Mutex<Box<dyn Adapter>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub description: DeviceDescription,
    properties: HashMap<String, Arc<Mutex<Box<dyn PropertyBase>>>>,
    actions: HashMap<String, Arc<Mutex<Box<dyn ActionBase>>>>,
    events: HashMap<String, Arc<Mutex<Box<dyn EventHandleBase>>>>,
}

impl DeviceHandle {
    pub(crate) fn new(
        client: Arc<Mutex<dyn Client>>,
        adapter: Weak<Mutex<Box<dyn Adapter>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        description: DeviceDescription,
    ) -> Self {
        DeviceHandle {
            client,
            weak: Weak::new(),
            adapter,
            plugin_id,
            adapter_id,
            description,
            device_id,
            properties: HashMap::new(),
            actions: HashMap::new(),
            events: HashMap::new(),
        }
    }

    pub(crate) fn add_property(&mut self, property_builder: Box<dyn PropertyBuilderBase>) {
        let name = property_builder.name();

        let property = Arc::new(Mutex::new(property_builder.build(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.device_id.clone(),
        )));

        self.properties.insert(name, property);
    }

    /// Get a reference to all the [properties][crate::property::Property] which this device owns.
    pub fn properties(&self) -> &HashMap<String, Arc<Mutex<Box<dyn PropertyBase>>>> {
        &self.properties
    }

    /// Get a [property][crate::property::Property] which this device owns by ID.
    pub fn get_property(&self, name: &str) -> Option<Arc<Mutex<Box<dyn PropertyBase>>>> {
        self.properties.get(name).cloned()
    }

    /// Set the value of a [property][crate::property::Property] which this device owns by ID.
    ///
    /// Make sure that the type of the provided value is compatible with the respective property.
    pub async fn set_property_value(
        &self,
        name: &str,
        value: Option<serde_json::Value>,
    ) -> Result<(), ApiError> {
        if let Some(property) = self.properties.get(name) {
            let mut property = property.lock().await;
            property.property_handle_mut().set_value(value).await?;
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

    /// Get a reference to all the [actions][crate::action::Action] which this device owns.
    pub fn actions(&self) -> &HashMap<String, Arc<Mutex<Box<dyn ActionBase>>>> {
        &self.actions
    }

    /// Get an [action][crate::action::Action] which this device owns by ID.
    pub fn get_action(&self, name: &str) -> Option<Arc<Mutex<Box<dyn ActionBase>>>> {
        self.actions.get(name).cloned()
    }

    pub(crate) async fn request_action(
        &self,
        action_name: String,
        action_id: String,
        input: serde_json::Value,
    ) -> Result<(), String> {
        let action = self.get_action(&action_name).ok_or_else(|| {
            format!(
                "Failed to request action {} of {}: not found",
                action_name, self.device_id,
            )
        })?;
        let mut action = action.lock().await;
        let action_handle = ActionHandle::new(
            self.client.clone(),
            self.weak.clone(),
            self.plugin_id.clone(),
            self.adapter_id.clone(),
            self.device_id.clone(),
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
            self.device_id.clone(),
            name.clone(),
        );

        let event = Arc::new(Mutex::new(event_handle));

        self.events.insert(name, event);
    }

    /// Get a reference to all the [events][crate::event::Event] which this device owns.
    pub fn events(&self) -> &HashMap<String, Arc<Mutex<Box<dyn EventHandleBase>>>> {
        &self.events
    }

    /// Get an [event][crate::event::Event] which this device owns by ID.
    pub fn get_event(&self, name: &str) -> Option<Arc<Mutex<Box<dyn EventHandleBase>>>> {
        self.events.get(name).cloned()
    }

    /// Raise an [event][crate::event::Event] which this device owns by ID.
    ///
    /// Make sure that the type of the provided data is compatible with the respective event.
    pub async fn raise_event(
        &self,
        name: &str,
        data: Option<serde_json::Value>,
    ) -> Result<(), ApiError> {
        if let Some(event) = self.events.get(name) {
            let event = event.lock().await;
            event.raise(data).await?;
            Ok(())
        } else {
            Err(ApiError::UnknownEvent)
        }
    }
}

/// A trait used to specify the structure of a WoT device.
///
/// Builds a [Device] instance. Created through an [adapter][crate::adapter::Adapter].
///
/// # Examples
/// ```
/// # #[macro_use]
/// # extern crate gateway_addon_rust;
/// # use gateway_addon_rust::{prelude::*, example::{ExampleDevice, ExamplePropertyBuilder, ExampleEvent, ExampleAction}};
/// # fn main() {}
/// // ...
/// pub struct ExampleDeviceBuilder();
///
/// impl DeviceBuilder for ExampleDeviceBuilder {
///     type Device = ExampleDevice;
///
///     fn id(&self) -> String {
///         "example-device".to_owned()
///     }
///
///     fn description(&self) -> DeviceDescription {
///         DeviceDescription::default()
///     }
///
///     fn properties(&self) -> Properties {
///         properties![ExamplePropertyBuilder::new()]
///     }
///
///     fn actions(&self) -> Actions {
///         actions![ExampleAction::new()]
///     }
///
///     fn events(&self) -> Events {
///         events![ExampleEvent::new()]
///     }
///
///     fn build(self, device_handle: DeviceHandle) -> Self::Device {
///         ExampleDevice::new(device_handle)
///     }
/// }
/// ```
pub trait DeviceBuilder: Send + Sync + 'static {
    /// Type of [device][Device] this builds.
    type Device: Device;

    /// ID of the device.
    fn id(&self) -> String;

    /// [WoT description][DeviceDescription] of the device.
    fn description(&self) -> DeviceDescription;

    /// A list of [properties][crate::property::PropertyBuilder] this device should own.
    ///
    /// Note that the desired list consists of boxed objects implementing [PropertyBuilderBase][crate::property::PropertyBuilderBase].
    /// You can use the convenienve macro [properties!][crate::properties] to create this list [PropertyBuilder][crate::property::PropertyBuilder]s.
    fn properties(&self) -> Properties {
        properties![]
    }

    /// A list of [actions][crate::action::Action] this device should own.
    ///
    /// Note that the desired list consists of boxed objects implementing [ActionBase][crate::action::ActionBase].
    /// You can use the convenienve macro [actions!][crate::actions] to create this list from [Action][crate::action::Action]s.
    fn actions(&self) -> Actions {
        actions![]
    }

    /// A list of [events][crate::event::Event] this device should own.
    ///
    /// Note that the desired list consists of boxed objects implementing [EventBase][crate::event::EventBase].
    /// You can use the convenienve macro [events!][crate::events] to create this list from [Event][crate::event::Event]s.
    fn events(&self) -> Events {
        events![]
    }

    /// Build a new instance of this device using the given [device handle][DeviceHandle].
    fn build(self, device_handle: DeviceHandle) -> Self::Device;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullDeviceDescription, ApiError> {
        let mut property_descriptions = BTreeMap::new();
        for property_builder in self.properties() {
            property_descriptions.insert(
                property_builder.name(),
                property_builder.full_description()?,
            );
        }

        let mut action_descriptions = BTreeMap::new();
        for action in self.actions() {
            action_descriptions.insert(action.name(), action.full_description());
        }

        let mut event_descriptions = BTreeMap::new();
        for event in self.events() {
            event_descriptions.insert(event.name(), event.full_description()?);
        }

        Ok(self.description().into_full_description(
            self.id(),
            property_descriptions,
            action_descriptions,
            event_descriptions,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        action::{Action, ActionHandle},
        action_description::{ActionDescription, NoInput},
        client::MockClient,
        device::DeviceHandle,
        device_description::DeviceDescription,
        event::Event,
        event_description::EventDescription,
        property::{Property, PropertyBuilder, PropertyHandle},
        property_description::PropertyDescription,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;

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
            EventDescription::default()
        }
    }

    #[test]
    fn test_add_property() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let client = Arc::new(Mutex::new(MockClient::new()));

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
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

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
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

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
            device_description,
        );

        device.add_event(Box::new(MockEvent::new(event_name.to_owned())));

        assert!(device.get_event(&event_name).is_some())
    }
}
