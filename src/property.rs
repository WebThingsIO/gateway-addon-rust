/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

pub use crate::property_description::*;
use crate::{api_error::ApiError, client::Client, device::Device};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DevicePropertyChangedNotificationMessageData, Message, Property as FullPropertyDescription,
};

#[async_trait]
pub trait Property: Send + Sync + 'static {
    type Value: Value;

    fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value>;

    async fn on_update(&mut self, _value: Self::Value) -> Result<(), String> {
        Ok(())
    }
}

#[async_trait]
pub trait PropertyBase: Send + Sync + AsAny + 'static {
    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase;

    #[doc(hidden)]
    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String>;
}

impl Downcast for dyn PropertyBase {}

#[async_trait]
impl<T: Property> PropertyBase for T {
    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase {
        <T as Property>::property_handle_mut(self)
    }
    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String> {
        let value = T::Value::deserialize(Some(value))
            .map_err(|err| format!("Could not deserialize value: {:?}", err))?;
        <T as Property>::on_update(self, value).await
    }
}

#[derive(Clone)]
pub struct PropertyHandle<T: Value> {
    client: Arc<Mutex<dyn Client>>,
    pub device: Weak<Mutex<Box<dyn Device>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub description: PropertyDescription<T>,
    _value: PhantomData<T>,
}

impl<T: Value> PropertyHandle<T> {
    pub(crate) fn new(
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: PropertyDescription<T>,
    ) -> Self {
        PropertyHandle {
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            description,
            _value: PhantomData,
        }
    }

    pub async fn set_value(&mut self, value: T) -> Result<(), ApiError> {
        self.description.value = value;

        let message: Message = DevicePropertyChangedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: self.device_id.clone(),
            property: self
                .description
                .clone()
                .into_full_description(self.name.clone())?,
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
}

#[async_trait]
pub trait PropertyHandleBase: Send + Sync + AsAny + 'static {
    async fn set_value(&mut self, value: Option<serde_json::Value>) -> Result<(), ApiError>;
}

impl Downcast for dyn PropertyHandleBase {}

#[async_trait]
impl<T: Value> PropertyHandleBase for PropertyHandle<T> {
    async fn set_value(&mut self, value: Option<serde_json::Value>) -> Result<(), ApiError> {
        let value = <T as Value>::deserialize(value)?;
        PropertyHandle::set_value(self, value).await
    }
}

pub trait PropertyBuilder: Send + Sync + 'static {
    type Property: Property<Value = Self::Value>;

    type Value: Value;

    fn name(&self) -> String;

    fn description(&self) -> PropertyDescription<Self::Value>;

    fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullPropertyDescription, ApiError> {
        self.description().into_full_description(self.name())
    }

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    fn build_(
        self: Box<Self>,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Self::Property {
        let property_handle = PropertyHandle::<Self::Value>::new(
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            self.name(),
            self.description(),
        );
        self.build(property_handle)
    }
}

pub trait PropertyBuilderBase: Send + Sync + 'static {
    fn name(&self) -> String;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullPropertyDescription, ApiError>;

    #[doc(hidden)]
    fn build(
        self: Box<Self>,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn PropertyBase>;
}

impl<T: PropertyBuilder> PropertyBuilderBase for T {
    fn name(&self) -> String {
        <T as PropertyBuilder>::name(self)
    }
    fn full_description(&self) -> Result<FullPropertyDescription, ApiError> {
        <T as PropertyBuilder>::full_description(self)
    }
    #[doc(hidden)]
    fn build(
        self: Box<Self>,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn PropertyBase> {
        Box::new(<T as PropertyBuilder>::build_(
            self, client, device, plugin_id, adapter_id, device_id,
        ))
    }
}

pub type Properties = Vec<Box<dyn PropertyBuilderBase>>;

#[macro_export]
macro_rules! properties [
    ($($e:expr),*) => ({
        let mut _temp = $crate::property::Properties::new();
        $(_temp.push(Box::new($e));)*
        _temp
    })
];

#[cfg(test)]
mod tests {
    use crate::{client::MockClient, property::PropertyHandle, PropertyDescription};
    use serde_json::json;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    #[tokio::test]
    async fn test_set_value() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let client = Arc::new(Mutex::new(MockClient::new()));
        let value = 42;

        let property_description = PropertyDescription::<i32>::default();

        let mut property = PropertyHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            property_name.clone(),
            property_description,
        );

        let expected_value = Some(json!(value.clone()));

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DevicePropertyChangedNotification(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                        && msg.data.property.name == Some(property_name.clone())
                        && msg.data.property.value == expected_value
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        property.set_value(value).await.unwrap();
    }
}
