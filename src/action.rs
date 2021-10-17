/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    action_description::{ActionDescription, Input},
    api_error::ApiError,
    client::Client,
    device::DeviceBase,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonschema::JSONSchema;
use serde_json::Value;
use std::{
    any::Any,
    sync::{Arc, Weak},
    time::SystemTime,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    Action as FullActionDescription, DeviceActionStatusNotificationMessageData,
};

#[async_trait]
pub trait Action: Send + Sized + 'static {
    type Input: Input;
    fn name(&self) -> String;
    fn description(&self) -> ActionDescription<Self::Input>;
    fn full_description(&self) -> FullActionDescription {
        FullActionDescription {
            at_type: self.description().at_type,
            description: self.description().description,
            input: self.description().input,
            links: self.description().links,
            title: self.description().title,
        }
    }
    async fn perform(&mut self, _action_handle: ActionHandle<Self::Input>) -> Result<(), String>;
    async fn check_and_perform(
        &mut self,
        action_handle: ActionHandle<Value>,
    ) -> Result<(), String> {
        if let Some(ref input_schema) = self.description().input {
            let schema = JSONSchema::compile(input_schema).map_err(|err| {
                format!(
                    "Failed to parse input schema for action {:?}: {:?}",
                    self.name(),
                    err
                )
            })?;
            schema.validate(&action_handle.input).map_err(|err| {
                format!(
                    "Failed to validate input for action {:?}: {:?}",
                    self.name(),
                    err.collect::<Vec<_>>()
                )
            })?;
        }
        let input = Self::Input::deserialize(action_handle.input.clone())
            .map_err(|err| format!("Could not deserialize input: {:?}", err))?;
        self.perform(ActionHandle::new(
            action_handle.client,
            action_handle.device,
            action_handle.plugin_id,
            action_handle.adapter_id,
            action_handle.device_id,
            action_handle.name,
            action_handle.id,
            input,
            action_handle.input,
        ))
        .await
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
pub trait ActionBase: Send {
    fn name(&self) -> String;
    fn full_description(&self) -> FullActionDescription;
    async fn check_and_perform(&mut self, action_handle: ActionHandle<Value>)
        -> Result<(), String>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[async_trait]
impl<T: Action> ActionBase for T {
    fn name(&self) -> String {
        T::name(self)
    }
    fn full_description(&self) -> FullActionDescription {
        T::full_description(self)
    }
    async fn check_and_perform(
        &mut self,
        action_handle: ActionHandle<Value>,
    ) -> Result<(), String> {
        T::check_and_perform(self, action_handle).await
    }
    fn as_any(&self) -> &dyn Any {
        T::as_any(self)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        T::as_any_mut(self)
    }
}

#[derive(Clone)]
pub enum Status {
    Created,
    Pending,
    Completed,
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match &self {
            Status::Created => "created",
            Status::Pending => "pending",
            Status::Completed => "completed",
        }
        .to_owned()
    }
}

#[derive(Clone)]
pub struct ActionHandle<T: Input> {
    client: Arc<Mutex<dyn Client>>,
    pub device: Weak<Mutex<Box<dyn DeviceBase>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub id: String,
    pub input: T,
    pub input_: Value,
    pub status: Status,
    pub time_requested: DateTime<Utc>,
    pub time_completed: Option<DateTime<Utc>>,
}

impl<I: Input> ActionHandle<I> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        id: String,
        input: I,
        input_: Value,
    ) -> Self {
        ActionHandle {
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            id,
            input,
            input_,
            status: Status::Created,
            time_requested: SystemTime::now().into(),
            time_completed: None,
        }
    }

    pub async fn start(&mut self) -> Result<(), ApiError> {
        self.status = Status::Pending;
        self.status_notify().await?;
        Ok(())
    }

    pub async fn finish(&mut self) -> Result<(), ApiError> {
        self.status = Status::Completed;
        self.time_completed = Some(SystemTime::now().into());
        self.status_notify().await?;
        Ok(())
    }

    pub async fn status_notify(&self) -> Result<(), ApiError> {
        let message = DeviceActionStatusNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            adapter_id: self.adapter_id.clone(),
            device_id: self.device_id.clone(),
            action: webthings_gateway_ipc_types::ActionDescription {
                id: self.id.clone(),
                input: Some(self.input_.clone()),
                name: self.name.clone(),
                status: self.status.to_string(),
                time_requested: self.time_requested.to_rfc3339(),
                time_completed: self.time_completed.map(|t| t.to_rfc3339()),
            },
        }
        .into();

        self.client.lock().await.send_message(&message).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{action::ActionHandle, action_description::NoInput, client::MockClient};
    use serde_json::json;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    #[tokio::test]
    async fn test_action_start() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let action_name = String::from("action_name");
        let action_id = String::from("action_id");
        let client = Arc::new(Mutex::new(MockClient::new()));
        let input = json!(null);

        let mut action = ActionHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            action_name.clone(),
            action_id.clone(),
            NoInput,
            input.clone(),
        );

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceActionStatusNotification(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                        && msg.data.action.name == action_name
                        && msg.data.action.id == action_id
                        && msg.data.action.input == Some(input.clone())
                        && msg.data.action.status == "pending"
                        && msg.data.action.time_completed == None
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        action.start().await.unwrap();
    }

    #[tokio::test]
    async fn test_action_finish() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let action_name = String::from("action_name");
        let action_id = String::from("action_id");
        let client = Arc::new(Mutex::new(MockClient::new()));
        let input = json!(null);

        let mut action = ActionHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            action_name.clone(),
            action_id.clone(),
            NoInput,
            input.clone(),
        );

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceActionStatusNotification(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                        && msg.data.action.name == action_name
                        && msg.data.action.id == action_id
                        && msg.data.action.input == Some(input.clone())
                        && msg.data.action.status == "completed"
                        && msg.data.action.time_completed.is_some()
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        action.finish().await.unwrap();
    }
}
