/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WoT events.

mod event_data;
mod event_description;
mod event_handle;
mod event_trait;

pub use event_data::*;
pub use event_description::*;
pub use event_handle::*;
pub use event_trait::*;

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
    pub use super::event_trait::tests::*;
}
