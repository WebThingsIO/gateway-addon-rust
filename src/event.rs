/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WoT events.

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

/// A trait used to specify the structure and behaviour of a WoT event.
///
/// Initialized with an [event handle][EventHandle].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, event::NoData};
/// # use async_trait::async_trait;
/// # use std::time::Duration;
/// # use tokio::time::sleep;
/// struct ExampleEvent();
///
/// #[async_trait]
/// impl Event for ExampleEvent {
///     type Data = NoData;
///
///     fn name(&self) -> String {
///         "example-event".to_owned()
///     }
///     fn description(&self) -> EventDescription<Self::Data> {
///         EventDescription::default()
///     }
///     fn init(&self, event_handle: EventHandle<Self::Data>) {
///         tokio::spawn(async move {
///             sleep(Duration::from_millis(1000)).await;
///             event_handle.raise(NoData).await.unwrap();
///         });
///     }
/// }
/// ```
pub trait Event: Send + Sync + 'static {
    /// Type of [data][Data] this event contains.
    type Data: Data;

    /// Name of the event.
    fn name(&self) -> String;

    /// [WoT description][EventDescription] of the event.
    fn description(&self) -> EventDescription<Self::Data>;

    /// Called once during initialization with an [event handle][EventHandle] which can later be used to raise event instances.
    fn init(&self, _event_handle: EventHandle<Self::Data>) {}

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullEventDescription, ApiError> {
        self.description().into_full_description(self.name())
    }

    #[doc(hidden)]
    fn build_event_handle(
        &self,
        client: Arc<Mutex<Client>>,
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

/// An object safe variant of [Event].
///
/// Auto-implemented for all objects which implement the [Event] trait.  **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [Event] implementation.
///
/// This can (in contrast to the [Event] trait) be used to store objects for dynamic dispatch.
pub trait EventBase: Send + Sync + AsAny + 'static {
    /// Name of the event.
    fn name(&self) -> String;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullEventDescription, ApiError>;

    #[doc(hidden)]
    fn build_event_handle(
        &self,
        client: Arc<Mutex<Client>>,
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
        client: Arc<Mutex<Client>>,
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
    pub async fn raise(&self, data: T) -> Result<(), ApiError> {
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

/// Convenience type for a collection of [EventBase].
pub type Events = Vec<Box<dyn EventBase>>;

/// Convenience macro for building an [Events].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, example::ExampleEvent};
/// events![ExampleEvent::new()]
/// # ;
/// ```
#[macro_export]
macro_rules! events [
    ($($e:expr),*) => ({
        let _temp: Events = vec![$(Box::new($e)),*];
        _temp
    })
];

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        client::Client,
        event::{Event, EventHandle},
        event_description::{Data, EventDescription, NoData},
    };
    use std::{
        marker::PhantomData,
        sync::{Arc, Weak},
    };
    use tokio::sync::Mutex;
    use webthings_gateway_ipc_types::Message;

    pub struct MockEvent<T: Data> {
        event_name: String,
        _data: PhantomData<T>,
    }

    impl<T: Data> MockEvent<T> {
        pub fn new(event_name: String) -> Self {
            Self {
                event_name,
                _data: PhantomData,
            }
        }
    }

    impl<T: Data> Event for MockEvent<T> {
        type Data = T;

        fn name(&self) -> String {
            self.event_name.clone()
        }

        fn description(&self) -> EventDescription<Self::Data> {
            EventDescription::default()
        }
    }

    async fn test_raise_event<T: Data + PartialEq>(data: T) {
        let plugin_id = String::from("plugin_id");
        let adapter_id = String::from("adapter_id");
        let device_id = String::from("device_id");
        let event_name = String::from("event_name");
        let client = Arc::new(Mutex::new(Client::new()));

        let event_description = EventDescription::default();

        let event = EventHandle::<T>::new(
            client.clone(),
            Weak::new(),
            plugin_id.clone(),
            adapter_id.clone(),
            device_id.clone(),
            event_name.clone(),
            event_description,
        );

        let expected_data = Data::serialize(data.clone()).unwrap();

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

    #[tokio::test]
    async fn test_raise_nodata() {
        test_raise_event(NoData).await;
    }

    #[tokio::test]
    async fn test_raise_event_bool() {
        test_raise_event(true).await;
    }

    #[tokio::test]
    async fn test_raise_event_u8() {
        test_raise_event(142_u8).await;
    }

    #[tokio::test]
    async fn test_raise_event_i32() {
        test_raise_event(42).await;
    }

    #[tokio::test]
    async fn test_raise_event_f32() {
        test_raise_event(0.42_f32).await;
    }

    #[tokio::test]
    async fn test_raise_event_opti32() {
        test_raise_event(Some(42)).await;
        test_raise_event::<Option<i32>>(None).await;
    }

    #[tokio::test]
    async fn test_raise_event_string() {
        test_raise_event("foo".to_owned()).await;
    }
}
