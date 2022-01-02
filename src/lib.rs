/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! This crate makes it possible to write addons for the WebthingsIO gateway in Rust.
//!
//! To get started, have a look at a [complete example](https://github.com/WebThingsIO/example-adapter-rust).

pub mod action;
pub mod adapter;
pub mod api_handler;
#[doc(hidden)]
pub mod client;
pub mod database;
pub mod device;
pub mod error;
pub mod event;
#[cfg(not(test))]
#[cfg(debug_assertions)]
#[doc(hidden)]
pub mod example;
pub(crate) mod message_handler;
pub mod plugin;
pub mod property;
pub mod type_;

/// The purpose of this module is to condense imports almost every addon requires.
///
/// # Examples
/// ```
/// # #![allow(unused_imports)]
/// use gateway_addon_rust::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        action::{self, Action, ActionDescription, ActionHandle, Actions},
        actions,
        adapter::{adapter, Adapter, AdapterHandle},
        device::{device, Device, DeviceDescription, DeviceHandle, DeviceStructure},
        event::{self, Event, EventDescription, EventHandle, Events},
        events,
        plugin::Plugin,
        properties,
        property::{
            self, Properties, Property, PropertyBuilder, PropertyDescription, PropertyHandle,
        },
    };
}

pub use prelude::*;
