/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{action_description::ActionDescription, api_error::ApiError, client::Client};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonschema::JSONSchema;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    Action as FullActionDescription, DeviceActionStatusNotificationMessageData,
};

#[async_trait]
pub trait Action: Send {
    type Input: DeserializeOwned + JsonSchema + Send;
    fn name(&self) -> String;
    fn description(&self) -> ActionDescription<Self::Input>;
    async fn perform(&mut self, _action_handle: ActionHandle<Self::Input>) -> Result<(), String>;
    async fn check_and_perform(
        &mut self,
        action_handle: ActionHandle<Value>,
    ) -> Result<(), String> {
        if let Some(ref input_schema) = self.description().input {
            let schema = JSONSchema::compile(&input_schema).map_err(|err| {
                format!(
                    "Failed to parse input schema for action {:?}: {:?}",
                    self.name(),
                    err
                )
            })?;
            schema
                .validate(&action_handle.input)
                .map_err(|_| format!("Failed to validate input for action {:?}", self.name()))?;
        }
        let input: Self::Input =
            serde_json::from_value(action_handle.input.clone()).map_err(|err| {
                format!(
                    "Failed to parse input for action {:?}: {:?}",
                    self.name(),
                    err
                )
            })?;
        self.perform(ActionHandle::new(
            action_handle.client,
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
}

#[async_trait]
pub trait ActionBase: Send {
    fn name(&self) -> String;
    fn full_description(&self) -> FullActionDescription;
    async fn check_and_perform(&mut self, action_handle: ActionHandle<Value>)
        -> Result<(), String>;
}

#[async_trait]
impl<T: Action> ActionBase for T {
    fn name(&self) -> String {
        T::name(self)
    }

    async fn check_and_perform(
        &mut self,
        action_handle: ActionHandle<Value>,
    ) -> Result<(), String> {
        T::check_and_perform(self, action_handle).await
    }

    fn full_description(&self) -> FullActionDescription {
        FullActionDescription {
            at_type: self.description().at_type,
            description: self.description().description,
            input: self.description().input,
            links: self.description().links,
            title: self.description().title,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct NoInput;

impl<'de> Deserialize<'de> for NoInput {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(NoInput)
    }
}

impl JsonSchema for NoInput {
    fn schema_name() -> String {
        "no input".to_owned()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
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
pub struct ActionHandle<T: DeserializeOwned> {
    client: Arc<Mutex<dyn Client>>,
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

impl<I: DeserializeOwned> ActionHandle<I> {
    pub fn new(
        client: Arc<Mutex<dyn Client>>,
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
            plugin_id,
            adapter_id,
            device_id,
            name,
            id,
            input: input,
            input_: input_,
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
