/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    client::Client,
    error::WebthingsError,
    event::{Data, EventBase},
    Device, Event, EventDescription, EventHandle,
};
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;
use webthings_gateway_ipc_types::Event as FullEventDescription;

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
/// impl EventStructure for ExampleEvent {
///     type Data = NoData;
///
///     fn name(&self) -> String {
///         "example-event".to_owned()
///     }
///     fn description(&self) -> EventDescription<Self::Data> {
///         EventDescription::default()
///     }
/// }
/// ```
pub trait EventStructure: Send + Sync + 'static {
    /// Type of [data][Data] this event contains.
    type Data: Data;

    /// Name of the event.
    fn name(&self) -> String;

    /// [WoT description][EventDescription] of the event.
    fn description(&self) -> EventDescription<Self::Data>;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullEventDescription, WebthingsError> {
        self.description().into_full_description(self.name())
    }
}

/// A trait used to build an [Event] around a data struct and a [event handle][EventHandle].
///
/// When you use the [event][macro@crate::event] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, event::{BuiltEvent, EventBuilder}};
/// # use async_trait::async_trait;
/// struct ExampleEvent {
///     foo: i32,
/// }
///
/// struct BuiltExampleEvent {
///     data: ExampleEvent,
///     event_handle: EventHandle<i32>,
/// }
///
/// impl BuiltEvent for BuiltExampleEvent {
///     // ...
///   # type Data = i32;
///   # fn event_handle(&self) -> &EventHandle<i32> {
///   #     &self.event_handle
///   # }
///   # fn event_handle_mut(&mut self) -> &mut EventHandle<i32> {
///   #     &mut self.event_handle
///   # }
/// }
///
/// impl EventStructure for ExampleEvent {
///     /// ...
/// #   type Data = i32;
/// #   fn name(&self) -> String {
/// #       "example-event".to_owned()
/// #   }
/// #   fn description(&self) -> EventDescription<Self::Data> {
/// #       EventDescription::default()
/// #   }
/// }
///
/// #[async_trait]
/// impl Event for BuiltExampleEvent {}
///
/// impl EventBuilder for ExampleEvent {
///     type BuiltEvent = BuiltExampleEvent;
///     fn build(data: Self, event_handle: EventHandle<i32>) -> Self::BuiltEvent {
///         BuiltExampleEvent {
///             data,
///             event_handle,
///         }
///     }
/// }
/// ```
pub trait EventBuilder: EventStructure {
    /// Type of [Event] to build.
    type BuiltEvent: Event;

    /// Build the [event][Event] from a data struct and an [event handle][EventHandle].
    fn build(
        data: Self,
        event_handle: EventHandle<<Self as EventStructure>::Data>,
    ) -> Self::BuiltEvent;
}

/// An object safe variant of [EventBuilder] + [EventStructure].
///
/// Auto-implemented for all objects which implement the [EventBuilder] trait. **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [EventBuilder] / [EventStructure] implementation.
///
/// This can (in contrast to to the [EventBuilder] trait) be used to store objects for dynamic dispatch.
pub trait EventBuilderBase: Send + Sync + 'static {
    /// Name of the event.
    fn name(&self) -> String;

    #[doc(hidden)]
    fn full_description(&self) -> Result<FullEventDescription, WebthingsError>;

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    fn build(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn EventBase>;
}

impl<T: EventBuilder> EventBuilderBase for T {
    fn name(&self) -> String {
        <T as EventStructure>::name(self)
    }

    fn full_description(&self) -> Result<FullEventDescription, WebthingsError> {
        <T as EventStructure>::full_description(self)
    }

    fn build(
        self: Box<Self>,
        client: Arc<Mutex<Client>>,
        device: Weak<Mutex<Box<dyn Device>>>,
        plugin_id: String,
        adapter_id: String,
        device_id: String,
    ) -> Box<dyn EventBase> {
        let event_handle = EventHandle::<<Self as EventStructure>::Data>::new(
            client,
            device,
            plugin_id,
            adapter_id,
            device_id,
            self.name(),
            self.description(),
        );
        Box::new(<T as EventBuilder>::build(*self, event_handle))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::ops::{Deref, DerefMut};

    use mockall::mock;

    use crate::{
        event::{tests::BuiltMockEvent, Data, EventBuilder},
        EventDescription, EventHandle, EventStructure,
    };

    mock! {
        pub EventHelper<T: Data> {
            pub fn post_init(&mut self);
        }
    }

    pub struct MockEvent<T: Data> {
        event_name: String,
        pub expect_post_init: bool,
        pub event_helper: MockEventHelper<T>,
    }

    impl<T: Data> MockEvent<T> {
        pub fn new(event_name: String) -> Self {
            Self {
                event_name,
                expect_post_init: false,
                event_helper: MockEventHelper::new(),
            }
        }
    }

    impl<T: Data> Deref for MockEvent<T> {
        type Target = MockEventHelper<T>;

        fn deref(&self) -> &Self::Target {
            &self.event_helper
        }
    }

    impl<T: Data> DerefMut for MockEvent<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.event_helper
        }
    }

    impl<T: Data> EventStructure for MockEvent<T> {
        type Data = T;

        fn name(&self) -> String {
            self.event_name.clone()
        }

        fn description(&self) -> EventDescription<Self::Data> {
            EventDescription::default()
        }
    }

    impl<T: Data> EventBuilder for MockEvent<T> {
        type BuiltEvent = BuiltMockEvent<T>;

        fn build(data: Self, event_handle: EventHandle<T>) -> Self::BuiltEvent {
            BuiltMockEvent::new(data, event_handle)
        }
    }
}
