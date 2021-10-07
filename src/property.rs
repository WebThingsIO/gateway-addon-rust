/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::{api_error::ApiError, client::Client, property_description::PropertyDescription};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{DevicePropertyChangedNotificationMessageData, Message};

#[async_trait]
pub trait Property: Send {
    fn borrow_property_handle(&mut self) -> &mut PropertyHandle;
    async fn on_update(&mut self, _value: Value) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct PropertyHandle {
    client: Arc<Mutex<dyn Client>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub description: PropertyDescription,
}

impl PropertyHandle {
    pub fn new(
        client: Arc<Mutex<dyn Client>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: PropertyDescription,
    ) -> Self {
        PropertyHandle {
            client,
            plugin_id,
            adapter_id,
            device_id,
            name,
            description,
        }
    }

    pub async fn set_value(&mut self, value: Value) -> Result<(), ApiError> {
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

#[cfg(test)]
mod tests {
    use crate::client::MockClient;
    use crate::property::PropertyHandle;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::{Message, Property as PropertyDescription};

    #[tokio::test]
    async fn test_unload() {
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
