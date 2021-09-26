/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use crate::client::Client;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DevicePropertyChangedNotificationMessageData, Message, Property as PropertyDescription,
};

#[async_trait(?Send)]
pub trait Property {
    fn borrow_property_handle(&mut self) -> &mut PropertyHandle;
    async fn on_update(&mut self, _value: Value) -> Result<(), String> {
        Ok(())
    }
}

pub struct PropertyHandle {
    client: Arc<Mutex<Client>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub description: PropertyDescription,
}

impl PropertyHandle {
    pub fn new(
        client: Arc<Mutex<Client>>,
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
