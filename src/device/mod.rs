/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WoT devices aka things.

mod device_builder;
mod device_description;
mod device_handle;
pub(crate) mod device_message_handler;
mod device_trait;

pub use device_builder::*;
pub use device_description::*;
pub use device_handle::*;
pub use device_trait::*;

#[cfg(test)]
pub(crate) mod tests {
    pub use super::{device_builder::tests::*, device_trait::tests::*};
}
