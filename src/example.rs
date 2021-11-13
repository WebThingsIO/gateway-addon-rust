/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    action::NoInput,
    actions,
    adapter::{Adapter, AdapterHandle},
    api_error::ApiError,
    event_description::NoData,
    events,
    plugin::connect,
    properties, Action, ActionDescription, ActionHandle, Actions, Device, DeviceBuilder,
    DeviceDescription, DeviceHandle, Event, EventDescription, Events, Properties, Property,
    PropertyBuilder, PropertyDescription, PropertyHandle,
};
use as_any::Downcast;
use async_trait::async_trait;

#[tokio::main]
pub async fn main() -> Result<(), ApiError> {
    let mut plugin = connect("example-addon").await?;
    let adapter = plugin
        .create_adapter("example-adapter", "Example Adapter", ExampleAdapter::new)
        .await?;
    adapter
        .lock()
        .await
        .downcast_mut::<ExampleAdapter>()
        .unwrap()
        .init()
        .await?;
    plugin.event_loop().await;
    Ok(())
}

pub struct ExampleAdapter(AdapterHandle);

impl Adapter for ExampleAdapter {
    fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
        &mut self.0
    }
}

impl ExampleAdapter {
    pub fn new(adapter_handle: AdapterHandle) -> Self {
        Self(adapter_handle)
    }

    async fn init(&mut self) -> Result<(), ApiError> {
        self.adapter_handle_mut()
            .add_device(ExampleDeviceBuilder::new())
            .await?;
        Ok(())
    }
}

pub struct ExampleDeviceBuilder();

impl DeviceBuilder for ExampleDeviceBuilder {
    type Device = ExampleDevice;

    fn id(&self) -> String {
        "example-device".to_owned()
    }

    fn description(&self) -> DeviceDescription {
        DeviceDescription::default()
    }

    fn properties(&self) -> Properties {
        properties![ExamplePropertyBuilder::new()]
    }

    fn actions(&self) -> Actions {
        actions![ExampleAction::new()]
    }

    fn events(&self) -> Events {
        events![ExampleEvent::new()]
    }

    fn build(self, device_handle: DeviceHandle) -> Self::Device {
        ExampleDevice::new(device_handle)
    }
}

impl ExampleDeviceBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self()
    }
}

pub struct ExampleDevice(DeviceHandle);

impl Device for ExampleDevice {
    fn device_handle_mut(&mut self) -> &mut DeviceHandle {
        &mut self.0
    }
}

impl ExampleDevice {
    pub fn new(device_handle: DeviceHandle) -> Self {
        Self(device_handle)
    }
}

pub struct ExamplePropertyBuilder();

impl PropertyBuilder for ExamplePropertyBuilder {
    type Property = ExampleProperty;
    type Value = i32;

    fn name(&self) -> String {
        "example-property".to_owned()
    }

    fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property {
        ExampleProperty::new(property_handle)
    }

    fn description(&self) -> PropertyDescription<i32> {
        PropertyDescription::default()
    }
}

impl ExamplePropertyBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self()
    }
}

pub struct ExampleProperty(PropertyHandle<i32>);

impl Property for ExampleProperty {
    type Value = i32;

    fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value> {
        &mut self.0
    }
}

impl ExampleProperty {
    pub fn new(property_handle: PropertyHandle<i32>) -> Self {
        Self(property_handle)
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
