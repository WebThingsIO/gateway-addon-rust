/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::{
    event::{Data, EventHandleBase},
    EventHandle,
};
use as_any::{AsAny, Downcast};

/// A trait used to specify the behaviour of a WoT event.
///
/// Built by a [crate::EventHandle].
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, event::NoData, event::BuiltEvent};
/// # use async_trait::async_trait;
/// # use std::time::Duration;
/// # use tokio::time::sleep;
/// #[event]
/// struct ExampleEvent {
///     foo: i32,
/// }
///
/// impl EventStructure for ExampleEvent {
///     // ...
///     # type Data = NoData;
///     # fn name(&self) -> String {
///     #     "example-event".to_owned()
///     # }
///     # fn description(&self) -> EventDescription<Self::Data> {
///     #     EventDescription::default()
///     # }
/// }
///
/// #[async_trait]
/// impl Event for BuiltExampleEvent {
///     fn post_init(&mut self) {
///         let event_handle = self.event_handle().clone();
///         tokio::task::spawn(async move {
///             sleep(Duration::from_millis(1000)).await;
///             event_handle.raise(NoData).await.unwrap();
///         });
///     }
/// }
/// ```
pub trait Event: BuiltEvent + Send + Sync + 'static {
    /// Called once after initialization.
    fn post_init(&mut self) {}
}

/// An object safe variant of [Event] + [BuiltEvent].
///
/// Auto-implemented for all objects which implement the [Event] trait.  **You never have to implement this trait yourself.**
///
/// Forwards all requests to the [Event] / [BuiltEvent] implementation.
///
/// This can (in contrast to the [Event] and [BuiltEvent] traits) be used to store objects for dynamic dispatch.
pub trait EventBase: Send + Sync + AsAny + 'static {
    /// Return a reference to the wrapped [event handle][EventHandle].
    fn event_handle(&self) -> &dyn EventHandleBase;

    /// Return a mutable reference to the wrapped [event handle][EventHandle].
    fn event_handle_mut(&mut self) -> &mut dyn EventHandleBase;

    #[doc(hidden)]
    fn post_init(&mut self);
}

impl Downcast for dyn EventBase {}

impl<T: Event> EventBase for T {
    fn event_handle(&self) -> &dyn EventHandleBase {
        <T as BuiltEvent>::event_handle(self)
    }

    fn event_handle_mut(&mut self) -> &mut dyn EventHandleBase {
        <T as BuiltEvent>::event_handle_mut(self)
    }

    fn post_init(&mut self) {
        <T as Event>::post_init(self)
    }
}

/// A trait used to wrap a [event handle][EventHandle].
///
/// When you use the [event][macro@crate::event] macro, this will be implemented automatically.
///
/// # Examples
/// ```
/// # use gateway_addon_rust::{prelude::*, event::{BuiltEvent, NoData}};
/// # use async_trait::async_trait;
/// struct BuiltExampleEvent {
///     event_handle: EventHandle<NoData>,
/// }
///
/// impl BuiltEvent for BuiltExampleEvent {
///     type Data = NoData;
///     fn event_handle(&self) -> &EventHandle<Self::Data> {
///         &self.event_handle
///     }
///     fn event_handle_mut(&mut self) -> &mut EventHandle<Self::Data> {
///         &mut self.event_handle
///     }
/// }
/// ```
pub trait BuiltEvent {
    /// Type of [data][Data] this event contains.
    type Data: Data;

    /// Return a reference to the wrapped [event handle][EventHandle].
    fn event_handle(&self) -> &EventHandle<Self::Data>;

    /// Return a mutable reference to the wrapped [event handle][EventHandle].
    fn event_handle_mut(&mut self) -> &mut EventHandle<Self::Data>;
}

#[cfg(test)]
pub(crate) mod tests {
    use std::ops::{Deref, DerefMut};

    use crate::{
        event::{tests::MockEvent, BuiltEvent, Data},
        Event, EventHandle,
    };

    pub struct BuiltMockEvent<T: Data> {
        data: MockEvent<T>,
        event_handle: EventHandle<T>,
    }

    impl<T: Data> BuiltMockEvent<T> {
        pub fn new(data: MockEvent<T>, event_handle: EventHandle<T>) -> Self {
            Self { data, event_handle }
        }
    }

    impl<T: Data> Deref for BuiltMockEvent<T> {
        type Target = MockEvent<T>;

        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    impl<T: Data> DerefMut for BuiltMockEvent<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    impl<T: Data> BuiltEvent for BuiltMockEvent<T> {
        type Data = T;

        fn event_handle(&self) -> &EventHandle<Self::Data> {
            &self.event_handle
        }

        fn event_handle_mut(&mut self) -> &mut EventHandle<Self::Data> {
            &mut self.event_handle
        }
    }

    impl<T: Data> Event for BuiltMockEvent<T> {
        fn post_init(&mut self) {
            if self.expect_post_init {
                self.event_helper.post_init();
            }
        }
    }
}
