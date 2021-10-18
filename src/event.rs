/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    api_error::ApiError,
    client::Client,
    device::DeviceBase,
    event_description::{Data, EventDescription},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::{
    any::Any,
    marker::PhantomData,
    sync::{Arc, Weak},
    time::SystemTime,
};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::{
    DeviceEventNotificationMessageData, Event as FullEventDescription, Message,
};

pub trait Event {
    type Data: Data + 'static;
    fn name(&self) -> String;
    fn description(&self) -> EventDescription<Self::Data>;
    fn full_description(&self) -> FullEventDescription {
        FullEventDescription {
            at_type: self.description().at_type,
            description: self.description().description,
            enum_: self.description().enum_,
            links: self.description().links,
            maximum: self.description().maximum,
            minimum: self.description().minimum,
            multiple_of: self.description().multiple_of,
            name: Some(self.name()),
            title: self.description().title,
            type_: self.description().type_,
            unit: self.description().unit,
        }
    }
    fn init(&self, _event_handle: EventHandle<Self::Data>) {}
    #[doc(hidden)]
    fn build_event_handle(
        &self,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
    ) -> EventHandle<Self::Data> {
        let event_handle =
            EventHandle::<Self::Data>::new(client, device, plugin_id, adapter_id, device_id, name);
        self.init(event_handle.clone());
        event_handle
    }
}

pub trait EventBase {
    fn name(&self) -> String;
    fn full_description(&self) -> FullEventDescription;
    #[doc(hidden)]
    fn build_event_handle(
        &self,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
    ) -> Box<dyn EventHandleBase>;
}

impl<T: Event> EventBase for T {
    fn name(&self) -> String {
        T::name(self)
    }

    fn full_description(&self) -> FullEventDescription {
        T::full_description(self)
    }

    fn build_event_handle(
        &self,
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
    ) -> Box<dyn EventHandleBase> {
        Box::new(T::build_event_handle(
            self, client, device, plugin_id, adapter_id, device_id, name,
        ))
    }
}

#[derive(Clone)]
pub struct EventHandle<D: Data> {
    client: Arc<Mutex<dyn Client>>,
    pub device: Weak<Mutex<Box<dyn DeviceBase>>>,
    pub plugin_id: String,
    pub adapter_id: String,
    pub device_id: String,
    pub name: String,
    _data: PhantomData<D>,
}

impl<D: Data + 'static> EventHandle<D> {
    pub(crate) fn new(
        client: Arc<Mutex<dyn Client>>,
        device: Weak<Mutex<Box<dyn DeviceBase>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
        name: String,
    ) -> Self {
        EventHandle {
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            name,
            _data: PhantomData,
        }
    }

    pub async fn raise(&self, data: D) -> Result<(), ApiError> {
        let data = Data::serialize(data)?;
        EventHandleBase::raise(self, data).await
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
pub trait EventHandleBase: Send + Sync {
    async fn raise(&self, data: Option<Value>) -> Result<(), ApiError>;
    fn as_any(&self) -> &dyn Any;
}

#[async_trait]
impl<D: Data + 'static> EventHandleBase for EventHandle<D> {
    async fn raise(&self, data: Option<Value>) -> Result<(), ApiError> {
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

    fn as_any(&self) -> &dyn Any {
        EventHandle::<D>::as_any(self)
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
    use crate::{client::MockClient, event::EventHandle};
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

        let event = EventHandle::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            event_name.clone(),
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
