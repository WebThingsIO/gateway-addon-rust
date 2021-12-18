/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{client::Client, error::WebthingsError, event::Data, Device, EventDescription};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
    time::SystemTime,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{DeviceEventNotificationMessageData, Message};

/// A struct which represents an instance of a WoT event.
///
/// Use it to notify the gateway.
#[derive(Clone)]
pub struct EventHandle<T: Data> {
    client: Arc<Mutex<Client>>,
    /// Reference to the [device][crate::device::Device] which owns this event.
    pub device: Weak<Mutex<Box<dyn Device>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    pub description: EventDescription<T>,
    _data: PhantomData<T>,
}

impl<T: Data> EventHandle<T> {
    pub(crate) fn new(
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
        description: EventDescription<T>,
    ) -> Self {
        EventHandle {
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            description,
            _data: PhantomData,
        }
    }

    /// Raise a new event instance of this event.
    pub async fn raise(&self, data: T) -> Result<(), WebthingsError> {
        let data = Data::serialize(data)?;
        EventHandleBase::raise(self, data).await
    }
}

/// A non-generic variant of [EventHandle].
///
/// Auto-implemented for all objects which implement the [EventHandle] trait. **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [EventHandle] implementation.
///
/// This can be used to store [event handles][EventHandle] with different data together.
#[async_trait]
pub trait EventHandleBase: Send + Sync + AsAny + 'static {
    /// Raise a new event instance of this event.
    ///
    /// Make sure that the type of the provided data is compatible.
    async fn raise(&self, data: Option<serde_json::Value>) -> Result<(), WebthingsError>;
}

impl Downcast for dyn EventHandleBase {}

#[async_trait]
impl<D: Data> EventHandleBase for EventHandle<D> {
    async fn raise(&self, data: Option<serde_json::Value>) -> Result<(), WebthingsError> {
        let time: DateTime<Utc> = SystemTime::now().into();
        let message: Message = DeviceEventNotificationMessageData {
            plugin_id: self.plugin_id.clone(),
            device_id: self.device_id.clone(),
            adapter_id: self.adapter_id.clone(),
            event: webthings_gateway_ipc_types::EventDescription {
                data,
                name: self.name.clone(),
                timestamp: time.to_rfc3339(),
            },
        }
        .into();

        self.client.lock().await.send_message(&message).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client::Client,
        event::{Data, NoData},
        EventDescription, EventHandle,
    };
    use rstest::rstest;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    const PLUGIN_ID: &str = "plugin_id";
    const ADAPTER_ID: &str = "adapter_id";
    const DEVICE_ID: &str = "device_id";
    const EVENT_NAME: &str = "event_name";

    #[rstest]
    #[case(NoData)]
    #[case(true)]
    #[case(142_u8)]
    #[case(42)]
    #[case(0.42_f32)]
    #[case(Some(42))]
    #[case(Option::<i32>::None)]
    #[case("foo".to_owned())]
    #[tokio::test]
    async fn test_raise_event<T: Data + PartialEq>(#[case] data: T) {
        let client = Arc::new(Mutex::new(Client::new()));

        let event_description = EventDescription::default();

        let event = EventHandle::<T>::new(
            client.clone(),
            Weak::new(),
            PLUGIN_ID.to_owned(),
            ADAPTER_ID.to_owned(),
            DEVICE_ID.to_owned(),
            EVENT_NAME.to_owned(),
            event_description,
        );

        let expected_data = Data::serialize(data.clone()).unwrap();

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceEventNotification(msg) => {
                    msg.data.plugin_id == PLUGIN_ID
                        && msg.data.adapter_id == ADAPTER_ID
                        && msg.data.device_id == DEVICE_ID
                        && msg.data.event.name == EVENT_NAME
                        && msg.data.event.data == expected_data
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        event.raise(data).await.unwrap();
    }
}
