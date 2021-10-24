/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

pub use crate::event_description::*;
use crate::{api_error::ApiError, client::Client, device::Device};
use as_any::{AsAny, Downcast};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
    time::SystemTime,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DeviceEventNotificationMessageData, Event as FullEventDescription, Message,
};

pub trait Event: Send + Sync + 'static {
    type Data: Data;

    fn name(&self) -> String;

    fn description(&self) -> EventDescription<Self::Data>;

    fn init(&self, _event_handle: EventHandle<Self::Data>) {}

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullEventDescription, ApiError> {
        self.description().into_full_description(self.name())
    }

    #[doc(hidden)]
    fn build_event_handle(
        &self,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
    ) -> EventHandle<Self::Data> {
        let event_handle = EventHandle::new(
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            self.description(),
        );
        self.init(event_handle.clone());
        event_handle
    }
}

pub trait EventBase: Send + Sync + AsAny + 'static {
    fn name(&self) -> String;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullEventDescription, ApiError>;

    #[doc(hidden)]
    fn build_event_handle(
        &self,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
    ) -> Box<dyn EventHandleBase>;
}

impl Downcast for dyn EventBase {}

impl<T: Event> EventBase for T {
    fn name(&self) -> String {
        <T as Event>::name(self)
    }

    fn full_description(&self) -> Result<FullEventDescription, ApiError> {
        <T as Event>::full_description(self)
    }

    fn build_event_handle(
        &self,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
    ) -> Box<dyn EventHandleBase> {
        Box::new(<T as Event>::build_event_handle(
            self, client, device, plugin_id, adapter_id, device_id, name,
        ))
    }
}

#[derive(Clone)]
pub struct EventHandle<T: Data> {
    client: Arc<Mutex<dyn Client>>,
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
        client: Arc<Mutex<dyn Client>>,
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

    pub async fn raise(&self, data: T) -> Result<(), ApiError> {
        let data = Data::serialize(data)?;
        EventHandleBase::raise(self, data).await
    }
}

#[async_trait]
pub trait EventHandleBase: Send + Sync + AsAny + 'static {
    async fn raise(&self, data: Option<serde_json::Value>) -> Result<(), ApiError>;
}

impl Downcast for dyn EventHandleBase {}

#[async_trait]
impl<D: Data> EventHandleBase for EventHandle<D> {
    async fn raise(&self, data: Option<serde_json::Value>) -> Result<(), ApiError> {
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

pub type Events = Vec<Box<dyn EventBase>>;

#[macro_export]
macro_rules! events [
    ($($e:expr),*) => ({
        let mut _temp = $crate::event::Events::new();
        $(_temp.push(Box::new($e));)*
        _temp
    })
];

#[cfg(test)]
mod tests {
    use crate::{client::MockClient, event::EventHandle, event_description::EventDescription};
    use serde_json::json;
    use std::sync::{Arc, Weak};
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    #[tokio::test]
    async fn test_raise_event() {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let event_name = String::from("event_name");
        let client = Arc::new(Mutex::new(MockClient::new()));
        let data = json!(42);

        let event_description = EventDescription::default();

        let event = EventHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            event_name.clone(),
            event_description,
        );

        let expected_data = Some(data.clone());

        client
            .lock()
            .await
            .expect_send_message()
            .withf(move |msg| match msg {
                Message::DeviceEventNotification(msg) => {
                    msg.data.plugin_id == plugin_id
                        && msg.data.adapter_id == adapter_id
                        && msg.data.device_id == device_id
                        && msg.data.event.name == event_name.clone()
                        && msg.data.event.data == expected_data
                }
                _ => false,
            })
            .times(1)
            .returning(|_| Ok(()));

        event.raise(data).await.unwrap();
    }
}
