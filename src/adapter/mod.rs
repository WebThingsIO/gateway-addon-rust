/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! A module for everything related to WebthingsIO adapters.

mod adapter_handle;
mod adapter_macros;
pub(crate) mod adapter_message_handler;
mod adapter_trait;

pub use adapter_handle::*;
pub use adapter_macros::*;
pub use adapter_trait::*;

#[cfg(test)]
pub(crate) mod tests {
    pub use super::{adapter_handle::tests::*, adapter_trait::tests::*};
}
