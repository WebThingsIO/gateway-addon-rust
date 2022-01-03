/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

#![allow(clippy::new_without_default)]

use crate::{
    action::NoInput,
    actions,
    adapter::{AdapterHandleWrapper, BuildAdapter},
    device::{BuildDevice, DeviceHandleWrapper},
    error::WebthingsError,
    event::NoData,
    events,
    plugin::connect,
    properties,
    property::{BuildProperty, PropertyHandleWrapper},
    Action, ActionDescription, ActionHandle, Actions, Adapter, AdapterHandle, Device,
    DeviceDescription, DeviceHandle, DeviceStructure, Event, EventDescription, Events, Properties,
    Property, PropertyDescription, PropertyHandle, PropertyStructure,
};
use as_any::Downcast;
use async_trait::async_trait;

#[tokio::main]
pub async fn main() -> Result<(), WebthingsError> {
    let mut plugin = connect("example-addon").await?;
    let adapter = plugin
        .create_adapter("example-adapter", "Example Adapter", ExampleAdapter::new())
        .await?;
    adapter
        .lock()
        .await
        .downcast_mut::<BuiltExampleAdapter>()
        .unwrap()
        .init()
        .await?;
    plugin.event_loop().await;
    Ok(())
}

pub struct ExampleAdapter;

pub struct BuiltExampleAdapter {
    data: ExampleAdapter,
    adapter_handle: AdapterHandle,
}

impl BuildAdapter for ExampleAdapter {
    type BuiltAdapter = BuiltExampleAdapter;
    fn build(data: Self, adapter_handle: AdapterHandle) -> Self::BuiltAdapter {
        BuiltExampleAdapter {
            data,
            adapter_handle,
        }
    }
}

impl AdapterHandleWrapper for BuiltExampleAdapter {
    fn adapter_handle(&self) -> &AdapterHandle {
        &self.adapter_handle
    }

    fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
        &mut self.adapter_handle
    }
}

impl std::ops::Deref for BuiltExampleAdapter {
    type Target = ExampleAdapter;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for BuiltExampleAdapter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Adapter for BuiltExampleAdapter {}

impl ExampleAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl BuiltExampleAdapter {
    async fn init(&mut self) -> Result<(), WebthingsError> {
        self.adapter_handle_mut()
            .add_device(ExampleDevice::new())
            .await?;
        Ok(())
    }
}

pub struct ExampleDevice;

pub struct BuiltExampleDevice {
    data: ExampleDevice,
    device_handle: DeviceHandle,
}

impl BuildDevice for ExampleDevice {
    type BuiltDevice = BuiltExampleDevice;
    fn build(data: Self, device_handle: DeviceHandle) -> Self::BuiltDevice {
        BuiltExampleDevice {
            data,
            device_handle,
        }
    }
}

impl DeviceHandleWrapper for BuiltExampleDevice {
    fn device_handle(&self) -> &DeviceHandle {
        &self.device_handle
    }

    fn device_handle_mut(&mut self) -> &mut DeviceHandle {
        &mut self.device_handle
    }
}

impl std::ops::Deref for BuiltExampleDevice {
    type Target = ExampleDevice;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for BuiltExampleDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl DeviceStructure for ExampleDevice {
    fn id(&self) -> String {
        "example-device".to_owned()
    }

    fn description(&self) -> DeviceDescription {
        DeviceDescription::default()
    }

    fn properties(&self) -> Properties {
        properties![ExampleProperty::new()]
    }

    fn actions(&self) -> Actions {
        actions![ExampleAction::new()]
    }

    fn events(&self) -> Events {
        events![ExampleEvent::new()]
    }
}

impl Device for BuiltExampleDevice {}

impl ExampleDevice {
    pub fn new() -> Self {
        Self
    }
}

pub struct ExampleProperty;

pub struct BuiltExampleProperty {
    data: ExampleProperty,
    property_handle: PropertyHandle<<ExampleProperty as PropertyStructure>::Value>,
}

impl BuildProperty for ExampleProperty {
    type BuiltProperty = BuiltExampleProperty;
    fn build(
        data: Self,
        property_handle: PropertyHandle<<Self as PropertyStructure>::Value>,
    ) -> Self::BuiltProperty {
        BuiltExampleProperty {
            data,
            property_handle,
        }
    }
}

impl PropertyHandleWrapper for BuiltExampleProperty {
    type Value = <ExampleProperty as PropertyStructure>::Value;

    fn property_handle(&self) -> &PropertyHandle<Self::Value> {
        &self.property_handle
    }

    fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value> {
        &mut self.property_handle
    }
}

impl std::ops::Deref for BuiltExampleProperty {
    type Target = ExampleProperty;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for BuiltExampleProperty {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl PropertyStructure for ExampleProperty {
    type Value = i32;

    fn name(&self) -> String {
        "example-Property".to_owned()
    }

    fn description(&self) -> PropertyDescription<Self::Value> {
        PropertyDescription::default()
    }
}

impl Property for BuiltExampleProperty {}

impl ExampleProperty {
    pub fn new() -> Self {
        Self
    }
}

pub struct ExampleAction();

#[async_trait]
impl Action for ExampleAction {
    type Input = NoInput;

    fn name(&self) -> String {
        "example-action".to_owned()
    }

    fn description(&self) -> ActionDescription<Self::Input> {
        ActionDescription::default()
    }

    async fn perform(&mut self, _action_handle: ActionHandle<Self::Input>) -> Result<(), String> {
        Ok(())
    }
}

impl ExampleAction {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self()
    }
}

pub struct ExampleEvent();

impl Event for ExampleEvent {
    type Data = NoData;

    fn name(&self) -> String {
        "example-event".to_owned()
    }

    fn description(&self) -> EventDescription<Self::Data> {
        EventDescription::default()
    }
}

impl ExampleEvent {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self()
    }
}
