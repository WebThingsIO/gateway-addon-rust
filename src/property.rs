/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::{
    api_error::ApiError,
    client::Client,
    device::DeviceBase,
    property_description::{PropertyDescription, Value},
};
use async_trait::async_trait;
use std::{
    any::Any,
    marker::PhantomData,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DevicePropertyChangedNotificationMessageData, Message, Property as FullPropertyDescription,
};

#[async_trait]
pub trait Property: Send + Sized + 'static {
    type Value: Value;
    fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value>;
    async fn on_update(&mut self, _value: Self::Value) -> Result<(), String> {
        Ok(())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
pub trait PropertyBase: Send {
    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase;
    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[async_trait]
impl<T: Property> PropertyBase for T {
    fn property_handle_mut(&mut self) -> &mut dyn PropertyHandleBase {
        T::property_handle_mut(self)
    }
    async fn on_update(&mut self, value: serde_json::Value) -> Result<(), String> {
        let value = serde_json::from_value(value)
            .map_err(|err| format!("Could not deserialize value: {:?}", err))?;
        T::on_update(self, value).await
    }
    fn as_any(&self) -> &dyn Any {
        T::as_any(self)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        T::as_any_mut(self)
    }
}

#[derive(Clone)]
pub struct PropertyHandle<T: Value> {
    client: Arc<Mutex<dyn Client>>,
    pub device: Weak<Mutex<Box<dyn DeviceBase>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub description: FullPropertyDescription,
    pub _value: PhantomData<T>,
}

impl<T: Value> PropertyHandle<T> {
    pub(crate) fn new(
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: FullPropertyDescription,
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
        let value = serde_json::to_value(value).map_err(ApiError::Serialization)?;
        PropertyHandleBase::set_value(self, value).await
    }
}

#[async_trait]
pub trait PropertyHandleBase: Send {
    async fn set_value(&mut self, value: serde_json::Value) -> Result<(), ApiError>;
}

#[async_trait]
impl<T: Value> PropertyHandleBase for PropertyHandle<T> {
    async fn set_value(&mut self, value: serde_json::Value) -> Result<(), ApiError> {
        self.description.value = Some(value);

        let message: Message = DevicePropertyChangedNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: self.device_id.clone(),
            property: self.description.clone(),
        }
        .into();

        self.client.lock().await.send_message(&message).await
    }
}

pub trait PropertyBuilder {
    type Property: Property<Value = Self::Value>;
    type Value: Value;
    fn name(&self) -> String;
    fn description(&self) -> PropertyDescription<Self::Value>;
    fn full_description(&self) -> FullPropertyDescription {
        let description = self.description();

        FullPropertyDescription {
            at_type: description.at_type,
            description: description.description,
            enum_: description.enum_,
            links: description.links,
            maximum: description.maximum,
            minimum: description.minimum,
            multiple_of: description.multiple_of,
            read_only: description.read_only,
            title: description.title,
            type_: description.type_,
            unit: description.unit,
            value: description.value,
            visible: description.visible,
            name: Some(self.name()),
        }
    }
    fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property;
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    fn build_(
        self: Box<Self>,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: FullPropertyDescription,
    ) -> Self::Property {
        let property_handle = PropertyHandle::<Self::Value>::new(
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            description,
        );
        self.build(property_handle)
    }
}

pub trait PropertyBuilderBase {
    fn name(&self) -> String;
    fn full_description(&self) -> FullPropertyDescription;
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    fn build(
        self: Box<Self>,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: FullPropertyDescription,
    ) -> Box<dyn PropertyBase>;
}

impl<T: PropertyBuilder> PropertyBuilderBase for T {
    fn name(&self) -> String {
        T::name(self)
    }
    fn full_description(&self) -> FullPropertyDescription {
        T::full_description(self)
    }
    #[doc(hidden)]
    fn build(
        self: Box<Self>,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: FullPropertyDescription,
    ) -> Box<dyn PropertyBase> {
        Box::new(T::build_(
            self,
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            description,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{client::MockClient, property::PropertyHandle};
    use serde_json::json;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{Message, Property as PropertyDescription};

    #[tokio::test]
    async fn test_set_value() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let property_name = String::from("property_name");
        let client = Arc::new(Mutex::new(MockClient::new()));
        let value = json!(42);

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

        let mut property = PropertyHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            property_name.clone(),
            property_description,
        );

        let expected_value = Some(value.clone());

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
