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
    client: Arc<Mutex<Client>>,
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
        client: Arc<Mutex<Client>>,
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

    /// Helper method for setting the value of a [property][crate::property::Property] which this device owns by ID.
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
            Err(ApiError::UnknownProperty(name.to_owned()))
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

    /// Helper method for raising an [event][crate::event::Event] which this device owns by ID.
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
            Err(ApiError::UnknownEvent(name.to_owned()))
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
pub(crate) mod tests {
    use crate::{
        action::{tests::MockAction, Actions, NoInput},
        actions,
        client::Client,
        device::{Device, DeviceBuilder, DeviceHandle},
        device_description::DeviceDescription,
        event::{tests::MockEvent, Events},
        event_description::NoData,
        events, properties,
        property::{tests::MockPropertyBuilder, Properties},
    };
    use serde_json::json;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;

    pub struct MockDevice {
        device_handle: DeviceHandle,
    }

    impl MockDevice {
        pub fn new(device_handle: DeviceHandle) -> Self {
            MockDevice { device_handle }
        }
    }

    impl Device for MockDevice {
        fn device_handle_mut(&mut self) -> &mut DeviceHandle {
            &mut self.device_handle
        }
    }

    pub struct MockDeviceBuilder {
        device_id: String,
    }

    impl MockDeviceBuilder {
        pub fn new(device_id: String) -> Self {
            Self { device_id }
        }
    }

    impl DeviceBuilder for MockDeviceBuilder {
        type Device = MockDevice;

        fn id(&self) -> String {
            self.device_id.clone()
        }

        fn description(&self) -> DeviceDescription {
            DeviceDescription::default()
        }

        fn properties(&self) -> Properties {
            properties![
                MockPropertyBuilder::<bool>::new("property_bool".to_owned()),
                MockPropertyBuilder::<u8>::new("property_u8".to_owned()),
                MockPropertyBuilder::<i32>::new("property_i32".to_owned()),
                MockPropertyBuilder::<f32>::new("property_f32".to_owned()),
                MockPropertyBuilder::<Option<i32>>::new("property_opti32".to_owned()),
                MockPropertyBuilder::<String>::new("property_string".to_owned())
            ]
        }

        fn actions(&self) -> Actions {
            actions![
                MockAction::<NoInput>::new("action_noinput".to_owned()),
                MockAction::<bool>::new("action_bool".to_owned()),
                MockAction::<u8>::new("action_u8".to_owned()),
                MockAction::<i32>::new("action_i32".to_owned()),
                MockAction::<f32>::new("action_f32".to_owned()),
                MockAction::<Option<i32>>::new("action_opti32".to_owned()),
                MockAction::<String>::new("action_string".to_owned())
            ]
        }

        fn events(&self) -> Events {
            events![MockEvent::<NoData>::new("event_nodata".to_owned())]
        }

        fn build(self, device_handle: DeviceHandle) -> Self::Device {
            MockDevice::new(device_handle)
        }
    }

    #[test]
    fn test_get_property() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
            device_description,
        );

        device.add_property(Box::new(MockPropertyBuilder::<i32>::new(
            property_name.clone(),
        )));

        assert!(device.get_property(&property_name).is_some())
    }

    #[test]
    fn test_get_unknown_property() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
            device_description,
        );

        assert!(device.get_property(&property_name).is_none())
    }

    #[test]
    fn test_get_action() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let action_name = String::from("action_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
            device_description,
        );

        device.add_action(Box::new(MockAction::<NoInput>::new(action_name.to_owned())));

        assert!(device.get_action(&action_name).is_some())
    }

    #[test]
    fn test_get_unknown_action() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let action_name = String::from("action_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
            device_description,
        );

        assert!(device.get_action(&action_name).is_none())
    }

    #[test]
    fn test_get_event() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let event_name = String::from("event_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
            device_description,
        );

        device.add_event(Box::new(MockEvent::<NoData>::new(event_name.to_owned())));

        assert!(device.get_event(&event_name).is_some())
    }

    #[test]
    fn test_get_unknown_event() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let event_name = String::from("event_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let device = DeviceHandle::new(
            client,
            Weak::new(),
            plugin_id,
            adapter_id,
            device_id,
            device_description,
        );

        assert!(device.get_event(&event_name).is_none())
    }

    #[tokio::test]
    async fn test_set_property_value() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let value = 42;
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            device_description,
        );

        device.add_property(Box::new(MockPropertyBuilder::<i32>::new(
            property_name.clone(),
        )));

        client
            .lock()
            .await
            .expect_send_message()
            .returning(|_| Ok(()));

        assert!(device
            .set_property_value(&property_name, Some(json!(value)))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_set_unknown_property_value() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let value = 42;
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let device = DeviceHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            device_description,
        );

        assert!(device
            .set_property_value(&property_name, Some(json!(value)))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_raise_event() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let event_name = String::from("event_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let mut device = DeviceHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            device_description,
        );

        device.add_event(Box::new(MockEvent::<NoData>::new(event_name.clone())));

        client
            .lock()
            .await
            .expect_send_message()
            .returning(|_| Ok(()));

        assert!(device.raise_event(&event_name, None).await.is_ok());
    }

    #[tokio::test]
    async fn test_raise_unknown_event() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let event_name = String::from("event_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let device_description = DeviceDescription::default();

        let device = DeviceHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            device_description,
        );

        assert!(device.raise_event(&event_name, None).await.is_err());
    }
}
