/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
pub mod action;
mod action_description;
pub mod adapter;
pub mod api_error;
pub(crate) mod client;
pub mod database;
pub mod device;
mod device_description;
pub mod event;
mod event_description;
pub mod plugin;
pub mod property;
mod property_description;
pub mod type_;

pub mod prelude {
    pub use crate::{
        action::{Action, ActionDescription, ActionHandle, Actions},
        actions,
        adapter::{Adapter, AdapterHandle},
        api_error::ApiError,
        database::Database,
        device::{Device, DeviceBuilder, DeviceDescription, DeviceHandle},
        event::{Event, EventDescription, EventHandle, Events},
        events,
        plugin::Plugin,
        properties,
        property::{Properties, Property, PropertyBuilder, PropertyDescription, PropertyHandle},
    };
}

pub use prelude::*;
