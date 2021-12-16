/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{action::Input, api_error::ApiError, client::Client, Device};


use chrono::{DateTime, Utc};

use std::{
    sync::{Arc, Weak},
    time::SystemTime,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DeviceActionStatusNotificationMessageData,
};

/// A struct which represents an instance of a WoT action.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct ActionHandle<T: Input> {
    pub(crate) client: Arc<Mutex<Client>>,
    /// Reference to the [device][crate::device::Device] which owns this action.
    pub device: Weak<Mutex<Box<dyn Device>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub id: String,
    pub input: T,
    input_: serde_json::Value,
    pub status: Status,
    pub time_requested: DateTime<Utc>,
    pub time_completed: Option<DateTime<Utc>>,
}

impl<T: Input> ActionHandle<T> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        id: String,
        input: T,
        input_: serde_json::Value,
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

    /// Notify the gateway that execution of this action instance has started.
    pub async fn start(&mut self) -> Result<(), ApiError> {
        self.status = Status::Pending;
        self.status_notify().await?;
        Ok(())
    }

    /// Notify the gateway that execution of this action instance has finished.
    pub async fn finish(&mut self) -> Result<(), ApiError> {
        self.status = Status::Completed;
        self.time_completed = Some(SystemTime::now().into());
        self.status_notify().await?;
        Ok(())
    }

    async fn status_notify(&self) -> Result<(), ApiError> {
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

/// Possible states of an [action][ActionHandle].
#[derive(Debug, Clone)]
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

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        action::{NoInput},
        client::Client,
        ActionHandle,
    };
    
    
    use rstest::{fixture, rstest};
    use serde_json::json;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";
    const ACTION_NAME: &str = "action_name";
    const ACTION_ID: &str = "action_id";
    const PENDING: &str = "pending";
    const COMPLETED: &str = "completed";
    const INPUT: serde_json::Value = json!(null);

    #[fixture]
    fn action() -> ActionHandle<NoInput> {
        let client = Arc::new(Mutex::new(Client::new()));
        ActionHandle::new(
            client,
            Weak::new(),
            PLUGIN_ID.to_owned(),
            ADAPTER_ID.to_owned(),
            DEVICE_ID.to_owned(),
            ACTION_NAME.to_owned(),
            ACTION_ID.to_owned(),
            NoInput,
            INPUT,
        )
    }

    #[rstest]
    #[tokio::test]
    async fn test_action_start(mut action: ActionHandle<NoInput>) {
        let input = json!(null);

        action
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceActionStatusNotification(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.action.name == ACTION_NAME
                        && msg.data.action.id == ACTION_ID
                        && msg.data.action.input == Some(input.clone())
                        && msg.data.action.status == PENDING
                        && msg.data.action.time_completed == None
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        action.start().await.unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn test_action_finish(mut action: ActionHandle<NoInput>) {
        action
            .client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceActionStatusNotification(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.action.name == ACTION_NAME
                        && msg.data.action.id == ACTION_ID
                        && msg.data.action.input == Some(INPUT)
                        && msg.data.action.status == COMPLETED
                        && msg.data.action.time_completed.is_some()
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        action.finish().await.unwrap();
    }
}
