/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{client::Client, error::WebthingsError, property::Value, Device, PropertyDescription};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{DevicePropertyChangedNotificationMessageData, Message};

/// A struct which represents an instance of a WoT property.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct PropertyHandle<T: Value> {
    client: Arc<Mutex<Client>>,
    /// Reference to the [device][crate::Device] which owns this property.
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
        client: Arc<Mutex<Client>>,
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

    /// Sets the [value][Value] and notifies the gateway.
    pub async fn set_value(&mut self, value: T) -> Result<(), WebthingsError> {
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

/// A non-generic variant of [PropertyHandle].
///
/// Auto-implemented for every [PropertyHandle]. **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [PropertyHandle] implementation.
#[async_trait]
pub trait PropertyHandleBase: Send + Sync + AsAny + 'static {
    /// Sets the [value][Value] and notifies the gateway.
    ///
    /// Make sure that the type of the provided value is compatible.
    async fn set_value(&mut self, value: Option<serde_json::Value>) -> Result<(), WebthingsError>;
}

impl Downcast for dyn PropertyHandleBase {}

#[async_trait]
impl<T: Value> PropertyHandleBase for PropertyHandle<T> {
    async fn set_value(&mut self, value: Option<serde_json::Value>) -> Result<(), WebthingsError> {
        let value = <T as Value>::deserialize(value)?;
        PropertyHandle::set_value(self, value).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::{client::Client, property::Value, PropertyDescription, PropertyHandle};

    use rstest::rstest;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";
    const PROPERTY_NAME: &str = "property_name";

    #[rstest]
    #[case(true)]
    #[case(142_u8)]
    #[case(42)]
    #[case(0.42_f32)]
    #[case(Some(42))]
    #[case(Option::<i32>::None)]
    #[case("foo".to_owned())]
    #[tokio::test]
    async fn test_set_value<T: Value + PartialEq>(#[case] value: T) {
        let client = Arc::new(Mutex::new(Client::new()));

        let property_description = PropertyDescription::<T>::default();

        let mut property = PropertyHandle::new(
            client.clone(),
            Weak::new(),
            PLUGIN_ID.to_owned(),
            ADAPTER_ID.to_owned(),
            DEVICE_ID.to_owned(),
            PROPERTY_NAME.to_owned(),
            property_description,
        );

        let expected_value = Value::serialize(value.clone()).unwrap();

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DevicePropertyChangedNotification(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.property.name == Some(PROPERTY_NAME.to_owned())
                        && msg.data.property.value == expected_value
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        property.set_value(value.clone()).await.unwrap();

        assert!(property.description.value == value);
    }
}
