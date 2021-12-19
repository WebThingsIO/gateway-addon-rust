/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    client::Client, error::WebthingsError, event::Data, Device, EventDescription, EventHandle,
};
use as_any::{AsAny, Downcast};

use std::sync::{Arc, Weak};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::Event as FullEventDescription;

use super::EventHandleBase;

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
    fn full_description(&self) -> Result<FullEventDescription, WebthingsError> {
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
    fn full_description(&self) -> Result<FullEventDescription, WebthingsError>;

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

    fn full_description(&self) -> Result<FullEventDescription, WebthingsError> {
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

#[cfg(test)]
pub(crate) mod tests {
    use crate::{event::Data, Event, EventDescription};

    use std::marker::PhantomData;

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
}
